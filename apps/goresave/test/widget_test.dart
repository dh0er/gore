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
    // Desktop window size so the inventory/diagnostics accordion (which fills
    // the available height) has room to lay out.
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
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

    expect(find.text('Gothic Remake Savegame Editor'), findsOneWidget);
    expect(find.text('Die Welt der Verurteilten'), findsAtLeastNWidgets(1));
    expect(find.text('Overview'), findsOneWidget);
    expect(find.text('Public save name'), findsOneWidget);
    // Header pills summarise chapter, time played and map for the save.
    expect(find.text('Chapter 1'), findsOneWidget);
    expect(find.text('1h 56m'), findsOneWidget);
    expect(find.text('MainMap'), findsAtLeastNWidgets(1));
    expect(find.text('Profile 0'), findsOneWidget);

    // Format/save-kind details live in the collapsed diagnostics card.
    expect(find.text('Format'), findsNothing);
    await tester.tap(find.text('Diagnostics & details'));
    await tester.pumpAndSettle();
    expect(find.text('Format'), findsOneWidget);
    expect(find.text('Time played'), findsOneWidget);
    expect(find.text('Auto save'), findsOneWidget);
    expect(find.bySemanticsLabel('Screenshot for G1R-001'), findsWidgets);

    // Inspection JSON card exists and is collapsed by default.
    expect(find.text('Inspection JSON'), findsOneWidget);
    expect(find.text('Raw save inspection data'), findsOneWidget);
    // JSON content is not visible until expanded.
    expect(find.text('"format"'), findsNothing);
    // Expand the card and confirm JSON content appears.
    await tester.scrollUntilVisible(
      find.text('Inspection JSON'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.tap(find.text('Inspection JSON'));
    await tester.pumpAndSettle();
    expect(find.textContaining('"format"'), findsOneWidget);
    // Collapse it again.
    await tester.tap(find.text('Inspection JSON'));
    await tester.pumpAndSettle();

    // Global Save button starts disabled (no pending edits yet).
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Save'),
          )
          .onPressed,
      isNull,
    );

    // Edit the public save name — button label gains count.
    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Much Longer Save Name',
    );
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // Tap the global Save button.
    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final publicWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(publicWrite.payload['edits'], [
      {'path': 'public.m_PlayerSaveName', 'value': 'Much Longer Save Name'},
    ]);
    expect(publicWrite.payload['syncPersistentDataList'], isTrue);
    expect(publicWrite.payload['backup'], isTrue);

    // Button disabled again after save.
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Save'),
          )
          .onPressed,
      isNull,
    );

    await tester.tap(find.widgetWithText(Tab, 'Player'));
    await tester.pumpAndSettle();

    // Player summary card and name editor fields are deleted.
    expect(find.text('Player summary'), findsNothing);
    expect(find.text('Save version'), findsNothing);
    expect(find.text('Current world'), findsNothing);
    expect(find.text('Profile name'), findsNothing);
    expect(
      find.widgetWithText(TextField, 'Private player name'),
      findsNothing,
    );
    expect(
      find.widgetWithText(TextField, 'Private profile name'),
      findsNothing,
    );

    // No individual per-editor save buttons.
    expect(find.byTooltip('Save Health attribute'), findsNothing);
    expect(find.byTooltip('Save hero transform'), findsNothing);

    // Legacy path (no typedParse in fixture): attributes render inside their
    // own Card titled 'Hero attributes'.
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
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

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
    await tester.pump();

    // Two pending edits: attr:Health + transform.
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final combinedWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(combinedWrite.payload['backup'], isTrue);
    final edits = combinedWrite.payload['edits'] as List;
    // Stable key order: 'attr:Health' < 'transform'.
    expect(edits, hasLength(2));
    expect(edits[0]['path'], 'private.player.setAttribute');
    expect(
      edits[0]['value'],
      {'id': 'Health', 'baseValue': 77.0, 'currentValue': 66.0},
    );
    expect(edits[1]['path'], 'private.player.setTransform');
    expect(edits[1]['value'], {
      'location': {'x': 100.0, 'y': 200.0, 'z': 300.0},
      'rotation': {'pitch': 1.0, 'yaw': 2.0, 'roll': 3.0},
    });

    await tester.tap(find.widgetWithText(Tab, 'Inventory'));
    await tester.pumpAndSettle();

    expect(find.text('Observed item stacks'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsOneWidget);
    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('42'), findsAtLeastNWidgets(1));

    // No old per-item save buttons.
    expect(find.byTooltip('Save ItFo_Cheese count'), findsNothing);
    // No old batch save button text.
    expect(find.widgetWithText(FilledButton, 'Save 2 changes'), findsNothing);

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
    await tester.pump();

    // Both inventory edits are reflected in the global button count.
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final batchWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(batchWrite.payload['backup'], isTrue);
    final batchEdits = batchWrite.payload['edits'] as List;
    expect(batchEdits, hasLength(2));
    final batchPaths = batchEdits.map((e) => e['value']['id']).toList();
    expect(batchPaths, containsAll(['ItMi_Orenugget', 'ItFo_Cheese']));

    await tester.enterText(
      find.widgetWithText(TextField, 'Filter items'),
      'cheese',
    );
    await tester.pumpAndSettle();

    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsNothing);

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
      (r) => r.command == 'restore_backup',
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

  testWidgets(
    'switching tabs preserves unsaved edit and Save count',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
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

      // Enter a draft in the public name field on Overview.
      await tester.enterText(
        find.widgetWithText(TextField, 'Public save name'),
        'Draft Name',
      );
      await tester.pump();
      // Save button now shows 1 pending edit.
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to Player tab.
      await tester.tap(find.widgetWithText(Tab, 'Player'));
      await tester.pumpAndSettle();

      // Save count must still be 1 (tab switch must not drop pending edits).
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch back to Overview tab.
      await tester.tap(find.widgetWithText(Tab, 'Overview'));
      await tester.pumpAndSettle();

      // The draft text must still be visible in the field.
      final field = find.widgetWithText(TextField, 'Public save name');
      final editableText = tester.widget<EditableText>(
        find.descendant(of: field, matching: find.byType(EditableText)),
      );
      expect(editableText.controller.text, 'Draft Name');
      // Save button still shows 1.
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
    },
  );

  testWidgets('Reset button discards pending and restores field text',
      (tester) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
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

    // Confirm Reset is disabled with no pending edits.
    final resetFinder = find.widgetWithText(OutlinedButton, 'Reset');
    expect(resetFinder, findsOneWidget);
    expect(
      tester.widget<OutlinedButton>(resetFinder).onPressed,
      isNull,
    );

    // Enter a draft in the public name field.
    final originalName = 'Die Welt der Verurteilten';
    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Edited Name',
    );
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
    // Reset should now be enabled.
    expect(
      tester.widget<OutlinedButton>(resetFinder).onPressed,
      isNotNull,
    );

    // Tap Reset.
    await tester.tap(resetFinder);
    await tester.pumpAndSettle();

    // Pending count must be 0 and Reset disabled again.
    expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );
    expect(
      tester.widget<OutlinedButton>(resetFinder).onPressed,
      isNull,
    );

    // The field must display the canonical (original) name again.
    final field = find.widgetWithText(TextField, 'Public save name');
    final editableText = tester.widget<EditableText>(
      find.descendant(of: field, matching: find.byType(EditableText)),
    );
    expect(editableText.controller.text, originalName);
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
                'persistentProfileId': 0,
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
                'screenshot': {
                  'mimeType': 'image/png',
                  'byteLength': 68,
                  'bytesBase64':
                      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=',
                },
              },
            ],
            'profiles': [
              {
                'profileId': 0,
                'profileName': '0',
                'quickSaveSlots': ['G1R-001', 'G1R-002', 'G1R-003'],
                'autoSaveSlots': ['G1R-001', 'G1R-002'],
                'savedSlots': ['G1R-001'],
                'maxQuick': 3,
                'maxAuto': 2,
              },
            ],
            'activeProfileId': 0,
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
            'screenshot': {
              'mimeType': 'image/png',
              'byteLength': 68,
              'bytesBase64':
                  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=',
            },
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
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
            'adapter': 'pure_rust_kraken',
            'message': 'Codec host is ready.',
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
