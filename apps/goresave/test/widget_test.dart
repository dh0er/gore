import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

void main() {
  testWidgets('renders editor shell with fake save data', (tester) async {
    final core = _FakeCoreService();
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

    expect(find.text('goresave'), findsOneWidget);
    expect(find.text('Gothic Remake Savegame-Editor'), findsOneWidget);
    expect(find.text('Die Welt der Verurteilten'), findsAtLeastNWidgets(1));
    expect(find.text('Overview'), findsOneWidget);
    expect(find.text('Public save name'), findsOneWidget);
    expect(find.text('Chapter'), findsOneWidget);
    expect(find.text('MainMap'), findsOneWidget);
    expect(find.text('Time played'), findsOneWidget);
    expect(find.text('1h 56m'), findsOneWidget);
    expect(find.text('Auto save'), findsOneWidget);

    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Much Longer Save Name',
    );
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();

    final publicWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(publicWrite.payload['edits'], [
      {'path': 'public.m_PlayerSaveName', 'value': 'Much Longer Save Name'},
    ]);

    await tester.tap(find.widgetWithText(Tab, 'Player'));
    await tester.pumpAndSettle();

    expect(find.text('Player summary'), findsOneWidget);
    expect(find.text('Save version'), findsOneWidget);
    expect(find.text('17'), findsOneWidget);
    expect(find.text('Current world'), findsOneWidget);
    expect(find.text('WORLD'), findsOneWidget);
    expect(find.text('Hero'), findsAtLeastNWidgets(1));
    expect(find.text('Profile name'), findsOneWidget);
    expect(find.text('0'), findsAtLeastNWidgets(1));
    expect(find.text('Preview: 1 / 541 chunks'), findsNothing);
    expect(find.widgetWithText(FilledButton, 'Load all'), findsNothing);
    expect(
      find.widgetWithText(TextField, 'Private player name'),
      findsOneWidget,
    );

    await tester.enterText(
      find.widgetWithText(TextField, 'Private player name'),
      'Nameless',
    );
    await tester.scrollUntilVisible(
      find.byTooltip('Save private player name'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Save private player name'));
    await tester.pumpAndSettle();

    final playerWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(playerWrite.payload['edits'], [
      {'path': 'private.player.setPlayerName', 'value': 'Nameless'},
    ]);

    await tester.scrollUntilVisible(
      find.widgetWithText(TextField, 'Private profile name'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Private profile name'),
      'goresave',
    );
    await tester.scrollUntilVisible(
      find.byTooltip('Save private profile name'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Save private profile name'));
    await tester.pumpAndSettle();

    final profileWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(profileWrite.payload['edits'], [
      {'path': 'private.profile.setProfileName', 'value': 'goresave'},
    ]);

    await tester.scrollUntilVisible(
      find.text('Hero attributes'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.text('Health'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Health base'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Health current'), findsOneWidget);
    await tester.enterText(find.widgetWithText(TextField, 'Health base'), '77');
    await tester.enterText(
      find.widgetWithText(TextField, 'Health current'),
      '66',
    );
    await tester.tap(find.byTooltip('Save Health attribute'));
    await tester.pumpAndSettle();

    final attributeWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(attributeWrite.payload['edits'], [
      {
        'path': 'private.player.setAttribute',
        'value': {'id': 'Health', 'baseValue': 77.0, 'currentValue': 66.0},
      },
    ]);

    await tester.scrollUntilVisible(
      find.text('Hero transform'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.widgetWithText(TextField, 'Location X'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Location Y'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Location Z'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation pitch'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation yaw'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation roll'), findsOneWidget);

    await tester.enterText(find.widgetWithText(TextField, 'Location X'), '100');
    await tester.enterText(find.widgetWithText(TextField, 'Location Y'), '200');
    await tester.enterText(find.widgetWithText(TextField, 'Location Z'), '300');
    await tester.enterText(
      find.widgetWithText(TextField, 'Rotation pitch'),
      '1',
    );
    await tester.enterText(find.widgetWithText(TextField, 'Rotation yaw'), '2');
    await tester.enterText(
      find.widgetWithText(TextField, 'Rotation roll'),
      '3',
    );
    final saveHeroTransformButton = find.descendant(
      of: find.byTooltip('Save hero transform'),
      matching: find.byType(IconButton),
    );
    await tester.ensureVisible(saveHeroTransformButton);
    await tester.tap(saveHeroTransformButton, warnIfMissed: false);
    await tester.pumpAndSettle();

    final transformWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(transformWrite.payload['edits'], [
      {
        'path': 'private.player.setTransform',
        'value': {
          'location': {'x': 100.0, 'y': 200.0, 'z': 300.0},
          'rotation': {'pitch': 1.0, 'yaw': 2.0, 'roll': 3.0},
        },
      },
    ]);

    await tester.tap(find.widgetWithText(Tab, 'Inventory'));
    await tester.pumpAndSettle();

    expect(find.text('Inventory summary'), findsOneWidget);
    expect(find.text('Candidates'), findsOneWidget);
    expect(find.text('2'), findsAtLeastNWidgets(1));
    expect(find.text('ITMI_GOLD'), findsOneWidget);
    expect(find.text('BP_Item_Ore'), findsOneWidget);
    expect(find.text('Observed item stacks'), findsOneWidget);
    expect(find.text('Player inventory'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsOneWidget);
    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('42'), findsAtLeastNWidgets(1));

    final oreCountField = find.descendant(
      of: find.ancestor(
        of: find.text('ItMi_Orenugget'),
        matching: find.byType(ListTile),
      ),
      matching: find.widgetWithText(TextField, 'Count'),
    );
    await tester.enterText(oreCountField, '44');
    await tester.pump();
    final oreEditable = tester.widget<EditableText>(
      find.descendant(of: oreCountField, matching: find.byType(EditableText)),
    );
    expect(
      oreEditable.controller.selection,
      const TextSelection.collapsed(offset: 2),
    );

    await tester.enterText(
      find.descendant(
        of: find.ancestor(
          of: find.text('ItFo_Cheese'),
          matching: find.byType(ListTile),
        ),
        matching: find.widgetWithText(TextField, 'Count'),
      ),
      '7',
    );
    await tester.pumpAndSettle();

    expect(find.widgetWithText(FilledButton, 'Save 2 changes'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Save 2 changes'));
    await tester.pumpAndSettle();

    final batchWrite = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(batchWrite.payload['edits'], [
      {
        'path': 'private.inventory.setItemCount',
        'value': {
          'id': 'ItMi_Orenugget',
          'path': '/Script/Angelscript.ItMi_Orenugget',
          'count': 44,
        },
      },
      {
        'path': 'private.inventory.setItemCount',
        'value': {
          'id': 'ItFo_Cheese',
          'path': '/Script/Angelscript.ItFo_Cheese',
          'count': 7,
        },
      },
    ]);

    await tester.enterText(
      find.widgetWithText(TextField, 'Filter items'),
      'cheese',
    );
    await tester.pumpAndSettle();

    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsNothing);

    await tester.enterText(
      find.descendant(
        of: find.ancestor(
          of: find.text('ItFo_Cheese'),
          matching: find.byType(ListTile),
        ),
        matching: find.widgetWithText(TextField, 'Count'),
      ),
      '7',
    );
    await tester.scrollUntilVisible(
      find.byTooltip('Save ItFo_Cheese count'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byTooltip('Save ItFo_Cheese count'),
      warnIfMissed: false,
    );
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(write.payload['edits'], [
      {
        'path': 'private.inventory.setItemCount',
        'value': {
          'id': 'ItFo_Cheese',
          'path': '/Script/Angelscript.ItFo_Cheese',
          'count': 7,
        },
      },
    ]);

    await tester.tap(find.widgetWithText(Tab, 'Progression'));
    await tester.pumpAndSettle();

    expect(find.text('Progression summary'), findsOneWidget);
    expect(find.text('Generated events'), findsOneWidget);
    expect(find.text('Quest.Main.Chapter01'), findsOneWidget);
    expect(find.text('Dialog.Diego.IntroDone'), findsOneWidget);

    await tester.enterText(
      find.widgetWithText(TextField, 'Filter progression'),
      'dialog',
    );
    await tester.pumpAndSettle();

    expect(find.text('Dialog.Diego.IntroDone'), findsOneWidget);
    expect(find.text('Quest.Main.Chapter01'), findsNothing);

    await tester.drag(find.byType(TabBar), const Offset(-500, 0));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Backups'), warnIfMissed: false);
    await tester.pumpAndSettle();

    expect(find.text('G1R-001.sav.bak.200'), findsOneWidget);
    expect(find.text('Before edit'), findsOneWidget);

    await tester.tap(
      find.byTooltip('Restore G1R-001.sav.bak.200'),
      warnIfMissed: false,
    );
    await tester.pumpAndSettle();

    final restore = core.requests.lastWhere(
      (request) => request.command == 'restore_backup',
    );
    expect(restore.payload, {
      'path': r'C:\tmp\saves\G1R-001.sav',
      'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.200',
    });

    await tester.scrollUntilVisible(
      find.text('Companion backups'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();

    expect(find.text('Companion backups'), findsOneWidget);
    expect(find.text('PersistentDataList.sav.bak.250'), findsOneWidget);
    expect(find.text('Before companion edit'), findsOneWidget);
    expect(
      find.byTooltip('Restore PersistentDataList.sav.bak.250'),
      findsNothing,
    );
  });

  testWidgets('shows loading spinner in main editor view', (tester) async {
    final core = _SlowInspectCoreService();
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

    await tester.pump();

    expect(find.bySemanticsLabel('Loading editor data'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    core.completePending();
    await tester.pumpAndSettle();

    expect(find.bySemanticsLabel('Loading editor data'), findsNothing);
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

class _FakeCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'fake-core';

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
                'playerSaveName': 'Die Welt der Verurteilten',
                'persistentPlayerSaveName':
                    'Die Welt der Verurteilten, Tag 1, 13:07',
                'chapterId': 1,
                'mapName': 'MainMap',
                'timePlayedSeconds': 6963.34,
                'quickSave': false,
                'autoSave': true,
                'slotName': 'G1R-001',
                'compressionMethod': 'Oodle',
                'chunkCount': 451,
              },
            ],
          },
        };
      case 'inspect_save':
        final preview = payload.containsKey('privateChunkLimit');
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 914367,
            'sha1': 'abc',
            'trailerSize': 44,
            'public': {
              'slotName': 'G1R-001',
              'playerSaveName': 'Die Welt der Verurteilten',
            },
            'persistent': {
              'playerSaveName': 'Die Welt der Verurteilten, Tag 1, 13:07',
              'chapterId': 1,
              'mapName': 'MainMap',
              'timePlayedSeconds': 6963.34,
              'timeLoadedSeconds': 0.0,
              'quickSave': false,
              'autoSave': true,
              'profileId': 0,
            },
            'compressedStream': {
              'method': 'Oodle',
              'algorithmId': 2,
              'chunkCount': 451,
              'compressedSize': 905728,
              'uncompressedSize': 59049891,
              'trailingSize': 44,
            },
            'private': {
              'status': preview ? 'decoded_preview' : 'decoded',
              'message': preview
                  ? 'Private payload preview decoded through the G1R codec host.'
                  : 'Private payload decoded through the G1R codec host.',
              'preview': preview,
              'decodedChunkCount': preview ? 1 : null,
              'totalChunkCount': preview ? 541 : null,
              'decompressedSize': 59049891,
              'stringCount': preview ? 1 : 3,
              'strings': preview ? ['Hero'] : ['Hero', 'ChapterOne', 'OreBar'],
              'player': {
                'saveVersionNumber': 17,
                'currentWorld': 'WORLD',
                'playerName': 'Hero',
                'profileName': '0',
                'transform': {
                  'location': {'x': 10.0, 'y': 20.0, 'z': 30.0},
                  'rotation': {'pitch': 40.0, 'yaw': 50.0, 'roll': 60.0},
                },
                'attributes': [
                  {'id': 'Health', 'baseValue': 40.0, 'currentValue': 25.0},
                  {'id': 'Strength', 'baseValue': 10.0, 'currentValue': 10.0},
                ],
                'scriptPaths': ['/Script/Angelscript.GothicFinalDataGame'],
                'properties': ['m_SaveVersionNumber', 'm_CurrentWorld'],
                'writable': [
                  'private.player.setPlayerName',
                  'private.profile.setProfileName',
                  'private.player.setAttribute',
                  'private.player.setTransform',
                ],
              },
              'inventory': {
                'candidateCount': 2,
                'candidates': ['ITMI_GOLD', 'BP_Item_Ore'],
                'itemStackCount': 1,
                'itemScope': 'player_inventory_region',
                'items': [
                  {
                    'id': 'ItMi_Orenugget',
                    'path': '/Script/Angelscript.ItMi_Orenugget',
                    'count': 42,
                  },
                  {
                    'id': 'ItFo_Cheese',
                    'path': '/Script/Angelscript.ItFo_Cheese',
                    'count': 1,
                  },
                ],
                'scriptPaths': ['/Script/G1R.InventorySaveGameData'],
                'properties': ['m_InventoryItems', 'm_StackCount'],
                'writable': ['private.inventory.setItemCount'],
              },
              'progression': {
                'candidateCount': 3,
                'candidates': [
                  'Quest.Main.Chapter01',
                  'Dialog.Diego.IntroDone',
                  'Knowledge.OldCamp.PathKnown',
                ],
                'gameplayTags': [
                  'Quest.Main.Chapter01',
                  'Dialog.Diego.IntroDone',
                ],
                'sections': ['Generated events', 'Memorized events'],
                'scriptPaths': ['/Script/G1R.QuestSaveGameData'],
                'properties': ['m_GeneratedEvents', 'm_MemorizedEvents'],
              },
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav.bak.200',
                'fileName': 'G1R-001.sav.bak.200',
                'fileSize': 913000,
                'sha1': 'backup-sha',
                'createdEpoch': 200,
                'status': 'ok',
                'playerSaveName': 'Before edit',
              },
            ],
            'companionBackups': [
              {
                'path': r'C:\tmp\saves\PersistentDataList.sav.bak.250',
                'fileName': 'PersistentDataList.sav.bak.250',
                'fileSize': 4096,
                'sha1': 'persistent-backup-sha',
                'createdEpoch': 250,
                'status': 'ok',
                'scope': 'persistent_data_list',
                'slotName': 'G1R-001',
                'playerSaveName': 'Before companion edit',
              },
            ],
          },
        };
      case 'check_codec':
        return {
          'ok': true,
          'data': {
            'available': false,
            'canDecompress': false,
            'canCompress': false,
            'status': 'native_encoder_in_progress',
            'adapter': 'pure_rust_kraken',
            'message': 'Native encoder support is not available yet.',
          },
        };
      case 'write_save':
        final syncPersistent = payload['syncPersistentDataList'] == true;
        return {
          'ok': true,
          'data': {
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1',
            if (syncPersistent) ...{
              'persistentBackupPath':
                  r'C:\tmp\saves\PersistentDataList.sav.bak.2',
              'persistentBytesChanged': true,
            },
          },
        };
      case 'restore_backup':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'restoredFrom': payload['backupPath'],
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.300',
          },
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

class _SlowInspectCoreService extends _FakeCoreService {
  final _pending = <Completer<void>>[];

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'inspect_save') {
      final completer = Completer<void>();
      _pending.add(completer);
      await completer.future;
    }
    return super.execute(command, payload: payload);
  }

  void completePending() {
    for (final completer in _pending) {
      if (!completer.isCompleted) {
        completer.complete();
      }
    }
    _pending.clear();
  }
}
