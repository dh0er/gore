import 'support/shared_planner_core.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/ui/npc_world_repair_banner.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';

/// NPCs an older build revived without putting them back into the world must
/// be called out on the overview, and the repair must reach the core as one
/// `private.npc.revive` per NPC. A save without them must stay quiet.
void main() {
  const drake = 'OM_GRD_Drake_260-WorldPointActor_Drake';
  const guard = 'OC_GRD_Guard19_239-OC_Spawnpoint_Guard19_239';

  Future<void> pumpApp(WidgetTester tester, GoresaveCoreService core) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(withSharedPlanner(core)),
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

  testWidgets('stranded NPCs are flagged and revived again on save', (
    tester,
  ) async {
    final core = _NpcCoreService(strandedRevives: [drake, guard]);
    await pumpApp(tester, core);

    expect(find.byType(NpcWorldRepairBanner), findsOneWidget);
    expect(find.text('Revived NPCs missing from the world'), findsOneWidget);
    expect(find.textContaining('Affected: 2.'), findsOneWidget);

    // One press queues it — no confirmation step, Discard takes it back.
    await tester.tap(find.widgetWithText(FilledButton, 'Repair'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.textContaining('Repair queued'), findsOneWidget);
    await tester.tap(find.widgetWithText(TextButton, 'Discard'));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Repair'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Repair'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();
    final edits = [
      for (final write in core.requests.where((r) => r.command == 'write_save'))
        ...(write.payload['edits'] as List).cast<Map<String, Object?>>(),
    ];
    expect(edits.map((e) => e['path']).toSet(), {'private.npc.revive'});
    expect(edits.map((e) => (e['value'] as Map)['id']).toSet(), {drake, guard});
    // The save is re-inspected after writing, and the repaired NPCs are gone.
    expect(find.byType(NpcWorldRepairBanner), findsNothing);
  });

  testWidgets('without write capability the warning stays, the action goes', (
    tester,
  ) async {
    await pumpApp(
      tester,
      _NpcCoreService(strandedRevives: [drake], canCompress: false),
    );

    expect(find.text('Revived NPCs missing from the world'), findsOneWidget);
    expect(find.textContaining('cannot be written'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Repair'), findsNothing);
  });

  testWidgets('a save without stranded NPCs shows no banner', (tester) async {
    await pumpApp(tester, _NpcCoreService(strandedRevives: const []));

    expect(find.byType(NpcWorldRepairBanner), findsNothing);
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// A save whose NPC block reports `strandedRevives`.
class _NpcCoreService implements GoresaveCoreService {
  _NpcCoreService({required this.strandedRevives, this.canCompress = true});

  final List<String> strandedRevives;

  /// Whether the codec can compress; without it no private edit can be written.
  final bool canCompress;
  final requests = <_RecordedRequest>[];

  /// Whether a save went through; the repaired NPCs are no longer reported.
  bool _written = false;

  static const _cheesePath = '/Script/Angelscript.ItFo_Cheese';

  @override
  String get description => 'npc-world-repair-fake-core';

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
              'npc': {
                'hasNpcs': true,
                'writable': ['private.npc.revive'],
                'strandedRevives': _written
                    ? const <String>[]
                    : strandedRevives,
              },
              'inventory': {
                'itemStackCount': 1,
                'itemScope': 'player_inventory_region',
                'items': [
                  {
                    'id': 'ItFo_Cheese',
                    'path': _cheesePath,
                    'count': 3,
                    'removable': true,
                    'slotId': 0,
                    'containerType': 'MainContainer',
                  },
                ],
                'mainContainerPaths': [_cheesePath],
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
            'canCompress': canCompress,
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
                'count': 1,
                'removable': true,
                'slotId': 0,
                'containerType': 'MainContainer',
              },
            ],
            'mainContainerPaths': [_cheesePath],
            'writable': ['private.inventory.setItemCount'],
          },
        };
      case 'private.npc.attributes':
        return {
          'ok': true,
          'data': {'attributes': <Object?>[]},
        };
      case 'write_save':
        _written = true;
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
