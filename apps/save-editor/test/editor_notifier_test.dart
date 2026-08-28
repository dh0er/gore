import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/glossary_models.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/domain/story_state_models.dart';
import 'package:goresave/l10n/app_localizations_de.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/providers/data_providers.dart';

bool _sameTestPath(String a, String b) =>
    a.replaceAll('/', '\\').toLowerCase() ==
    b.replaceAll('/', '\\').toLowerCase();

void main() {
  test(
    'direct notifier construction defaults domain messages to English',
    () async {
      final notifier = EditorNotifier(_RecordingCoreService());

      await pumpEventQueue();

      final result = await notifier.loadSkills();
      expect(result.error, AppLocalizationsEn().editorNoSaveSelected);
    },
  );

  test(
    'provider notifier reads a changed locale without being recreated',
    () async {
      final container = ProviderContainer(
        overrides: [
          coreServiceProvider.overrideWithValue(_RecordingCoreService()),
          uiSettingsStoreProvider.overrideWithValue(
            const NoopUiSettingsStore(),
          ),
        ],
      );
      addTearDown(container.dispose);
      final notifier = container.read(editorProvider.notifier);

      await pumpEventQueue();
      container.read(localeProvider.notifier).setLocale('de');

      expect(container.read(editorProvider.notifier), same(notifier));
      final result = await notifier.loadSkills();
      expect(result.error, AppLocalizationsDe().editorNoSaveSelected);
    },
  );

  test('uses persisted save dir before defaults', () {
    final core = _RecordingCoreService();
    final store = _MemoryEditorSettingsStore(
      const EditorSettings(saveDir: r'D:\G1R\Saves'),
    );

    final notifier = EditorNotifier(core, settingsStore: store);

    expect(notifier.state.saveDir, r'D:\G1R\Saves');
  });

  test('setSaveDir persists editor settings', () async {
    final core = _RecordingCoreService();
    final store = _MemoryEditorSettingsStore();
    final notifier = EditorNotifier(core, settingsStore: store);

    await notifier.setSaveDir(r'E:\G1R\Saved\SaveGames');

    expect(store.settings.saveDir, r'E:\G1R\Saved\SaveGames');
  });

  test('checkCodec sends no codec configuration payload', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );

    await notifier.checkCodec();

    final checkCodec = core.requests.lastWhere(
      (request) => request.command == 'check_codec',
    );
    expect(checkCodec.payload.containsKey('binaryHost'), isFalse);
    expect(checkCodec.payload, isEmpty);
  });

  test(
    'refresh parses profiles, screenshots, and sends no codec config',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
              'screenshot': {
                'mimeType': 'image/jpeg',
                'byteLength': 6,
                'bytesBase64': '/9gBAv/Z',
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
            },
          ],
          'activeProfileId': 0,
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );

      await pumpEventQueue();

      final scan = core.requests.firstWhere(
        (request) => request.command == 'scan_save_dir',
      );
      expect(scan.payload.containsKey('binaryHost'), isFalse);
      expect(scan.payload, {'path': r'C:\tmp\saves'});
      expect(notifier.state.profiles.single.displayName, 'Profile 0');
      expect(notifier.state.activeProfile?.profileId, 0);
      expect(notifier.state.selectedSave?.screenshot?.byteLength, 6);
    },
  );

  test(
    'activeResourcesLevel normalizes known levels and falls back to Gothic',
    () async {
      // Active profile carries a Resources difficulty class ending in
      // '_Hard' (the raw class-name shape DifficultySettings.resourcesLabel
      // expects — see _difficultyLevelLabel), which resourcesLabel maps to
      // the normalized label 'Hard'.
      final core = _RecordingCoreService(
        scanData: {
          'saves': <Object?>[],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'customResourcesSettings': 'ResourcesDifficultySettings_Hard',
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');

      await pumpEventQueue();

      expect(notifier.state.activeProfile?.difficulty.resourcesLabel, 'Hard');
      expect(notifier.activeResourcesLevel(), 'Hard');

      // No active profile at all (empty scan) → falls back to 'Gothic'.
      final core2 = _RecordingCoreService();
      final notifier2 = EditorNotifier(core2, saveDir: r'C:\tmp\saves');

      await pumpEventQueue();

      expect(notifier2.state.activeProfile, isNull);
      expect(notifier2.activeResourcesLevel(), 'Gothic');
    },
  );

  test(
    'activeResourcesLevel uses the preset-implied level when no explicit sub-level',
    () async {
      // A non-Custom preset stores NO explicit Resources sub-level (resourcesLabel
      // is '-'); the level is implied by the preset. Reading only resourcesLabel
      // would wrongly send every Novice/Hard profile to the Gothic start-save.
      Future<EditorNotifier> build(Map<String, Object?> profileFields) async {
        final core = _RecordingCoreService(
          scanData: {
            'saves': <Object?>[],
            'profiles': [
              {'profileId': 0, 'profileName': '0', ...profileFields},
            ],
            'activeProfileId': 0,
          },
        );
        final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
        await pumpEventQueue();
        return notifier;
      }

      // Novice preset (class suffix '_Easy'), no explicit resources → 'Novice'.
      final novice = await build({'difficultyPreset': 'DifficultyPreset_Easy'});
      expect(novice.state.activeProfile?.difficulty.resourcesLabel, '-');
      expect(novice.activeResourcesLevel(), 'Novice');

      // Hard preset ('_Hard') → 'Hard'.
      final hard = await build({'difficultyPreset': 'DifficultyPreset_Hard'});
      expect(hard.activeResourcesLevel(), 'Hard');

      // Gothic preset ('_Standard') → 'Gothic'.
      final gothic = await build({
        'difficultyPreset': 'DifficultyPreset_Standard',
      });
      expect(gothic.activeResourcesLevel(), 'Gothic');

      // Custom preset with an explicit Resources sub-level → the explicit level
      // wins over the preset ('_Easy' resources class → 'Novice').
      final custom = await build({
        'difficultyPreset': 'DifficultyPreset_Custom',
        'customResourcesSettings': 'ResourcesDifficultySettings_Easy',
      });
      expect(custom.activeResourcesLevel(), 'Novice');

      // A non-Custom preset LOCKS the level: a stale/disagreeing stored Resources
      // class is ignored (Hard preset + stale '_Standard' resources → 'Hard', NOT
      // 'Gothic'). Only Custom profiles let the sub-level override the preset.
      final hardStale = await build({
        'difficultyPreset': 'DifficultyPreset_Hard',
        'customResourcesSettings': 'ResourcesDifficultySettings_Standard',
      });
      expect(hardStale.activeResourcesLevel(), 'Hard');
    },
  );

  test('inspect sends no codec config and decodes all chunks', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );

    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final inspect = core.requests.lastWhere(
      (request) => request.command == 'inspect_save',
    );
    expect(inspect.payload.containsKey('privateChunkLimit'), isFalse);
    expect(inspect.payload.containsKey('binaryHost'), isFalse);
    expect(notifier.state.backups.single.fileName, 'G1R-001.sav.bak.200');
    expect(notifier.state.backups.single.playerSaveName, 'Before edit');
    expect(
      notifier.state.companionBackups.single.fileName,
      'PersistentDataList.sav.bak.250',
    );
    // Companion (PersistentDataList.sav) backups are restorable directly.
    expect(notifier.state.companionBackups.single.canRestore, isTrue);
  });

  test('restoreBackup sends backup path and refreshes selected save', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    await notifier.restoreBackup(r'C:\tmp\saves\G1R-001.sav.bak.200');

    final restore = core.requests.lastWhere(
      (request) => request.command == 'restore_backup',
    );
    expect(restore.payload, {
      'path': r'C:\tmp\saves\G1R-001.sav',
      'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.200',
    });
    expect(
      notifier.state.lastWriteMessage,
      contains(r'Restored backup: C:\tmp\saves\G1R-001.sav.bak.200'),
    );
  });

  // ---------------------------------------------------------------------------
  // Pending-edit registry
  // ---------------------------------------------------------------------------

  test('setPendingEdit adds entry and updates count', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
        syncPersistentDataList: true,
      ),
    );

    expect(notifier.state.pendingEdits.containsKey('publicName'), isTrue);
    expect(notifier.pendingEditCount, 1);
  });

  test('clearPendingEdit removes entry', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );
    expect(notifier.pendingEditCount, 1);

    notifier.clearPendingEdit('publicName');
    expect(notifier.state.pendingEdits, isEmpty);
    expect(notifier.pendingEditCount, 0);
  });

  test('invalid NPC edit blocks Save while keeping the stored draft', () {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');

    // A valid pending NPC attribute edit is registered.
    notifier.setPendingEdit(
      'npc.attributes:Lizard-1',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {'path': 'Strength', 'value': 50},
          },
        ],
      ),
    );
    expect(notifier.state.hasInvalidNpcEdit, isFalse);

    // The field goes invalid: Save is blocked, but the stored draft survives so
    // switching actors does not silently lose it.
    notifier.setNpcEditInvalid('npc.attributes:Lizard-1');
    expect(notifier.state.hasInvalidNpcEdit, isTrue);
    expect(
      notifier.state.pendingEdits.containsKey('npc.attributes:Lizard-1'),
      isTrue,
    );

    // Valid again → unblocked.
    notifier.setNpcEditInvalid(null);
    expect(notifier.state.hasInvalidNpcEdit, isFalse);

    // Switching actor also abandons the invalid in-progress field → unblocked.
    notifier.setNpcEditInvalid('npc.attributes:Lizard-1');
    notifier.selectActor(
      const Actor.npc(id: 'Lizard-2', name: 'L2', uniqueName: 'Lizard'),
    );
    expect(notifier.state.hasInvalidNpcEdit, isFalse);
  });

  test('legacy invalidNpcEditKey remains a compatible state channel', () {
    final state = EditorState(
      saveDir: r'C:\tmp\saves',
      invalidNpcEditKey: 'legacy-npc-draft',
    );

    expect(state.invalidNpcEditKey, 'legacy-npc-draft');
    expect(state.invalidEditKeys, {'legacy-npc-draft'});
    final cleared = state.copyWith(invalidNpcEditKey: null);
    expect(cleared.invalidNpcEditKey, isNull);
    expect(cleared.invalidEditKeys, isEmpty);
  });

  test(
    'saveAllPending issues ONE write_save with mixed edits in stable key order',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Register two pending edits with keys that sort: 'attr:Health' < 'transform'
      notifier.setPendingEdit(
        'transform',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.player.setTransform',
              'value': {
                'location': {'x': 1.0, 'y': 2.0, 'z': 3.0},
                'rotation': {'pitch': 0.0, 'yaw': 0.0, 'roll': 0.0},
              },
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'attr:Health',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.player.setAttribute',
              'value': {
                'id': 'Health',
                'baseValue': 77.0,
                'currentValue': 66.0,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writeRequests = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Exactly one write_save.
      expect(writeRequests, hasLength(1));
      final payload = writeRequests.single.payload;
      expect(payload['backup'], isTrue);
      // Edits in stable key order: 'attr:Health' before 'transform'.
      final edits = payload['edits'] as List;
      expect(edits, hasLength(2));
      expect(edits[0]['path'], 'private.player.setAttribute');
      expect(edits[1]['path'], 'private.player.setTransform');
      // Pending cleared after success.
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test(
    'saveAllPending refuses a reset queued with a same-inventory typed edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'inventory',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.reset',
              'value': {'resourcesLevel': 'Gothic'},
            },
          ],
        ),
      );
      // A raw All-data edit stepping through the player's m_Inventory. Running
      // in the fixed batch before the reset splice, it would be silently
      // overwritten by the reset — so the save must be refused.
      notifier.setPendingEdit(
        'typed:inv',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': ['m_SavedPlayers', '[0]', 'm_Inventory', 'm_Keys'],
                'value': 0,
              },
            },
          ],
        ),
      );

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('discard'));
      // No write_save issued — the conflict is caught before the worklist runs.
      final writesAfter = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      expect(writesAfter, writesBefore);
      // Both pending edits are preserved for the user to resolve.
      expect(notifier.state.pendingEdits, isNotEmpty);
    },
  );

  test(
    'saveAllPending refuses a reset queued with a same-actor inventory count edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Reset the PLAYER inventory (no actorId)...
      notifier.setPendingEdit(
        'inventory',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.reset',
              'value': {'resourcesLevel': 'Gothic'},
            },
          ],
        ),
      );
      // ...and a PLAYER setItemCount (same actor: no actorId) under another key.
      // It lands in the fixed batch before the reset splice, so the reset would
      // discard it — the save must be refused.
      notifier.setPendingEdit(
        'count',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.setItemCount',
              'value': {'id': 'ItMi_Orenugget', 'count': 5},
            },
          ],
        ),
      );

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('discard'));
      expect(
        core.requests.where((r) => r.command == 'write_save').length,
        writesBefore,
      );
    },
  );

  test(
    'saveAllPending allows a reset with an inventory edit on a DIFFERENT actor',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Reset the PLAYER inventory...
      notifier.setPendingEdit(
        'inventory',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.reset',
              'value': {'resourcesLevel': 'Gothic'},
            },
          ],
        ),
      );
      // ...and a setItemCount on an NPC (different inventory) — no conflict, the
      // reset does not touch the NPC's inventory, so the save proceeds.
      notifier.setPendingEdit(
        'inventory:Char_1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.setItemCount',
              'value': {
                'id': 'ItMi_Orenugget',
                'count': 5,
                'actorId': 'Char_1',
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      // Not refused for the conflict — it committed both (recording core succeeds).
      expect(ok, isTrue);
      expect(notifier.state.error, isNull);
    },
  );

  test(
    'activeResourcesLevel follows the save/scan profile, not the sidebar filter',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': <Object?>[],
          'profiles': [
            {
              'profileId': 0,
              'profileName': 'A',
              'difficultyPreset': 'DifficultyPreset_Hard',
            },
            {
              'profileId': 1,
              'profileName': 'B',
              'difficultyPreset': 'DifficultyPreset_Easy',
            },
          ],
          // The scan's active (this save's) profile is 1 → Novice.
          'activeProfileId': 1,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      // The user explicitly filters the sidebar to profile 0 (Hard) — a browsing
      // choice that must NOT change which start-save a reset targets.
      notifier.selectProfile(0);
      expect(notifier.state.activeProfile?.difficulty.presetLabel, 'Hard');

      // Reset follows the save/scan profile (1 = Novice), not the filter.
      expect(notifier.activeResourcesLevel(), 'Novice');
    },
  );

  test(
    'activeResourcesLevel falls back to the inspected save own difficulty (no profile)',
    () async {
      const savePath = r'C:\tmp\saves\Standalone.sav';
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': savePath,
              // A standalone/imported save: no profile attribution, but the GSAV
              // still carries its own parsed difficulty (Hard).
              'persistentProfileId': null,
              'difficulty': {'preset': 'DifficultyPreset_Hard'},
            },
          ],
          'profiles': <Object?>[],
          'activeProfileId': null,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();
      await notifier.inspect(savePath);

      // No profile metadata resolves → use the save's OWN difficulty (Hard),
      // not the Gothic default.
      expect(notifier.state.selectedSave?.path, savePath);
      expect(notifier.state.activeProfile, isNull);
      expect(notifier.activeResourcesLevel(), 'Hard');
    },
  );

  test(
    'activeResourcesLevel prefers an unattributed save own difficulty over the active profile',
    () async {
      const savePath = r'C:\tmp\saves\Imported.sav';
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': savePath,
              'persistentProfileId': null, // not attached to any profile
              'difficulty': {
                'preset': 'DifficultyPreset_Hard',
              }, // save's own = Hard
            },
          ],
          // The folder HAS a profile and it is the scan-active one — but it is a
          // DIFFERENT (Novice) profile, not this unattributed save's.
          'profiles': [
            {
              'profileId': 7,
              'profileName': 'Other',
              'difficultyPreset': 'DifficultyPreset_Easy',
            },
          ],
          'activeProfileId': 7,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();
      await notifier.inspect(savePath);

      expect(notifier.state.selectedSave?.path, savePath);
      // Unattributed save → its OWN difficulty (Hard) wins over the active
      // profile's (Novice); we never borrow a different save's profile.
      expect(notifier.activeResourcesLevel(), 'Hard');
    },
  );

  test(
    'saveAllPending sets syncPersistentDataList true when any edit requests it',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
          ],
          syncPersistentDataList: true,
        ),
      );
      notifier.setPendingEdit(
        'attr:Health',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.player.setAttribute',
              'value': {
                'id': 'Health',
                'baseValue': 80.0,
                'currentValue': 80.0,
              },
            },
          ],
        ),
      );

      await notifier.saveAllPending();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      expect(write.payload['syncPersistentDataList'], isTrue);
      expect(write.payload['backup'], isTrue);
    },
  );

  test('saveAllPending is a no-op when pendingEdits is empty', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    final countBefore = core.requests
        .where((r) => r.command == 'write_save')
        .length;

    final ok = await notifier.saveAllPending();

    expect(ok, isTrue);
    final countAfter = core.requests
        .where((r) => r.command == 'write_save')
        .length;
    expect(countAfter, countBefore);
  });

  test('saveAllPending keeps pending edits on failure', () async {
    final core = _FailingWriteCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 914367,
            'sha1': 'abc',
            'status': 'ok',
            'playerSaveName': 'Auto',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );
    final scansBefore = core.commandCount('scan_save_dir');
    final inspectionsBefore = core.commandCount('inspect_save');
    final backupsBefore = core.commandCount('list_backups');

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('write failed'));
    expect(core.commandCount('scan_save_dir'), scansBefore + 1);
    expect(core.commandCount('inspect_save'), inspectionsBefore + 1);
    expect(core.commandCount('list_backups'), backupsBefore + 1);
    // Pending edits must be preserved so the user can retry.
    expect(notifier.state.pendingEdits.containsKey('publicName'), isTrue);
  });

  test('a failed save keeps the placement undo note with its edits', () async {
    // Regression: the pending entry was rebuilt from `edits` alone, so a retry
    // after a failed save would commit the NPC move WITHOUT recording the note —
    // pinning him with no way back to the routine the move replaced.
    final core = _FailingWriteCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 914367,
            'sha1': 'abc',
            'status': 'ok',
            'playerSaveName': 'Auto',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    notifier.setPendingEdit(
      'npc.position:A',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': ['x'],
              'value': {'x': 1.0, 'y': 2.0, 'z': 3.0},
            },
          },
        ],
        placementNotes: [
          {
            'npc': 'A',
            'note': {
              'original_location': [0.0, 0.0, 0.0],
              'original_routine_class': '/Script/Angelscript.DailyRoutine_A',
              'written_location': [1.0, 2.0, 3.0],
              'written_routine_class': '/Script/Angelscript.DailyRoutine_Empty',
            },
          },
        ],
        clearPlacementNotes: ['B'],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    final kept = notifier.state.pendingEdits['npc.position:A'];
    expect(kept, isNotNull);
    expect(kept!.placementNotes, hasLength(1));
    expect(kept.placementNotes.single['npc'], 'A');
    expect(kept.clearPlacementNotes, ['B']);
  });

  test('a failed undo note is reported beside the successful save', () async {
    // The core writes the note AFTER the bytes land and reports a failure beside
    // a good save rather than failing it. Left unread, the UI would announce
    // success and clear the drafts while the routine the pin replaced was gone
    // with nothing recording it.
    final core = _PlacementWarningCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 914367,
            'sha1': 'abc',
            'status': 'ok',
            'playerSaveName': 'Auto',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    notifier.setPendingEdit(
      'npc.position:A',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isTrue);
    expect(notifier.state.lastWriteMessage, contains('npc_placements.json'));
  });

  test('a restore says when the backup\'s undo notes did not follow', () async {
    // The bytes are the backup's either way; only the notes describing them
    // failed. Unreported, the restored save can hold a pinned NPC while the
    // sidecar says nothing about the routine that pin replaced.
    final core = _RestoreWarningCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 914367,
            'sha1': 'abc',
            'status': 'ok',
            'playerSaveName': 'Auto',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();

    await notifier.restoreBackup(r'C:\tmp\saves\G1R-001.sav.bak.1');

    expect(notifier.state.lastWriteMessage, contains('npc_placements.json'));
  });

  test('a failed undo note survives a later sub-write failing', () async {
    // The committed sub-write put the move on disk; a later failure does not
    // take it back. Reporting only the save error would leave an NPC pinned
    // with the routine it replaced recorded nowhere.
    final core = _WarnThenFailCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 914367,
            'sha1': 'abc',
            'status': 'ok',
            'playerSaveName': 'Auto',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await pumpEventQueue();
    // Batching packs peers into one write, so the two sub-writes this test needs
    // come from an ordinary edit plus a `private.story.apply`, which is
    // permanently exclusive. The revive commits (with the undo-note warning),
    // the trailing story write fails.
    notifier.setPendingEdit(
      'npc.revive:A',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.npc.revive',
            'value': {'id': 'A'},
          },
        ],
      ),
    );
    notifier.setStoryStateEdit(
      const StoryStateEdit(
        id: 'Chapter',
        present: true,
        rawValue: 3,
        expectedStored: true,
        expectedRawValue: 2,
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    // Two sub-writes really were issued — otherwise the fake never reaches its
    // failure branch and the assertion below would prove nothing.
    expect(
      core.requests.where((request) => request.command == 'write_save'),
      hasLength(2),
    );
    expect(notifier.state.error, contains('npc_placements.json'));
  });

  test(
    'saveAllPending on partial commit clears only the committed snapshot keys',
    () async {
      // The save still exists in the post-save scan, so refresh keeps it
      // selected and the uncommitted edit stays pending for retry.
      final core = _FailSecondWriteCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
            },
          ],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // An ordinary splicing edit plus a `private.story.apply`: story is
      // permanently exclusive, so the batch is packed into exactly two
      // sub-writes. The revive commits (first write), the story write fails.
      notifier.setPendingEdit(
        'npc.revive:A',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'A'},
            },
          ],
        ),
      );
      notifier.setStoryStateEdit(
        const StoryStateEdit(
          id: 'Chapter',
          present: true,
          rawValue: 3,
          expectedStored: true,
          expectedRawValue: 2,
        ),
      );

      final scansBefore = core.refreshScans;
      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, isNotNull);
      expect(
        core.requests.where((request) => request.command == 'write_save'),
        hasLength(2),
      );
      // First write committed → its key is cleared; the failed second's key stays.
      expect(notifier.state.pendingEdits.containsKey('npc.revive:A'), isFalse);
      expect(
        notifier.state.pendingEdits.containsKey(storyStatePendingKey),
        isTrue,
      );
      // The committed edit changed the file, so the panes must be refreshed from
      // disk even though a later sub-write failed. refresh() begins with a
      // scan_save_dir, so exactly one ADDITIONAL scan proves the partial-commit
      // refresh ran (vs. the old early-return that left the UI stale).
      expect(core.refreshScans, scansBefore + 1);
    },
  );

  test(
    'partial commit refreshes and preserves unwritten edits when second write throws',
    () async {
      final core = _ThrowSecondWriteCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
            },
          ],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      // Ordinary edit + exclusive `private.story.apply` = two sub-writes, so the
      // fake's throw lands on a genuine SECOND write after the first committed.
      notifier.setPendingEdit(
        'npc.revive:A',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'A'},
            },
          ],
        ),
      );
      notifier.setStoryStateEdit(
        const StoryStateEdit(
          id: 'Chapter',
          present: true,
          rawValue: 3,
          expectedStored: true,
          expectedRawValue: 2,
        ),
      );

      final scansBefore = core.refreshScans;
      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('native worker died'));
      expect(
        core.requests.where((request) => request.command == 'write_save'),
        hasLength(2),
      );
      expect(notifier.state.pendingEdits.containsKey('npc.revive:A'), isFalse);
      expect(
        notifier.state.pendingEdits.containsKey(storyStatePendingKey),
        isTrue,
      );
      expect(core.refreshScans, scansBefore + 1);
      expect(notifier.state.saveProgress, isNull);
    },
  );

  test(
    'partial commit keeps the uncommitted add when several share one key',
    () async {
      // Regression: one pending key can still span several sequential
      // write_saves — batching packs peers together, but an exclusive edit
      // always starts a new sub-write. Commit tracking is per-EDIT, so when the
      // later sub-write fails the earlier committed edit must not drag the
      // still-unwritten one out of pending — the user must keep it for retry.
      final core = _FailSecondWriteCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
            },
          ],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // An add and an exclusive `private.story.apply` under ONE key → two
      // sequential writes: the add commits, the story write fails.
      notifier.setPendingEdit(
        'inventory:player',
        PendingSaveEdit(
          edits: [
            const <String, Object?>{
              'path': 'private.inventory.addItem',
              'value': {'path': '/Game/Item_A', 'count': 1},
            },
            storyStateApplyEdit(const [
              StoryStateEdit(
                id: 'Chapter',
                present: true,
                rawValue: 3,
                expectedStored: true,
                expectedRawValue: 2,
              ),
            ]),
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, isNotNull);
      // The two edits of that one key really did split across two sub-writes.
      final writes = core.requests
          .where((request) => request.command == 'write_save')
          .toList();
      expect(writes, hasLength(2));
      expect(
        (writes.first.payload['edits'] as List).single,
        containsPair('path', 'private.inventory.addItem'),
      );
      expect(
        (writes.last.payload['edits'] as List).single,
        containsPair('path', storyStateApplyPath),
      );
      // The key survives, carrying ONLY the uncommitted story edit — the add
      // committed and is gone; the story edit never wrote and stays for retry.
      final pending = notifier.state.pendingEdits['inventory:player'];
      expect(pending, isNotNull);
      expect(pending!.edits, hasLength(1));
      expect(pending.edits.single['path'], storyStateApplyPath);
    },
  );

  test('saveAllPending clears the progress bar when a write throws', () async {
    // Regression: the save progress bar is set before the write loop. A thrown
    // write_save (CoreWorkerException from the worker isolate) must still clear
    // saveProgress, or a later load shows a determinate bar with stale counts.
    final core = _ThrowingWriteCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, isNotNull);
    expect(notifier.state.saveProgress, isNull);
    expect(notifier.state.isLoading, isFalse);
  });

  test(
    'partial commit drops uncommitted edits when the slot changes on refresh',
    () async {
      // After the partial commit the original save VANISHES from the scan
      // (only G1R-002 remains), so refresh auto-selects another slot. The
      // uncommitted edit targeted G1R-001 and must NOT be re-registered — else
      // the next Save would apply it to the wrong file.
      final core = _FailSecondWriteCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
            },
          ],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Ordinary edit + exclusive `private.story.apply` = two sub-writes; the
      // first commits, the second fails.
      notifier.setPendingEdit(
        'npc.revive:A',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'A'},
            },
          ],
        ),
      );
      notifier.setStoryStateEdit(
        const StoryStateEdit(
          id: 'Chapter',
          present: true,
          rawValue: 3,
          expectedStored: true,
          expectedRawValue: 2,
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(
        core.requests.where((request) => request.command == 'write_save'),
        hasLength(2),
      );
      // Slot switched to the only remaining save…
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
      // …so the uncommitted edit was dropped, NOT re-targeted at G1R-002.
      expect(
        notifier.state.pendingEdits.containsKey(storyStatePendingKey),
        isFalse,
      );
      expect(notifier.state.pendingEdits.containsKey('npc.revive:A'), isFalse);
    },
  );

  test('selection change clears pending edits', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );
    expect(notifier.state.pendingEdits.isNotEmpty, isTrue);

    // Inspect a different path — pending edits must be cleared.
    await notifier.inspect(r'C:\tmp\saves\G1R-002.sav');

    expect(notifier.state.pendingEdits, isEmpty);
  });

  test('re-inspecting the same save clears pending edits', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );
    expect(notifier.state.pendingEdits.isNotEmpty, isTrue);

    // Re-selecting the already-selected save re-seeds every editor from the
    // fresh inspection; stale registry entries must not survive it.
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    expect(notifier.state.pendingEdits, isEmpty);
  });

  test(
    'saveAllPending refuses conflicting edits for the same typed path',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      const path = ['m_GenericData', '{X}', 'BaseValue'];
      notifier.setPendingEdit(
        'heroStats',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {'path': path, 'value': 1.0},
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'typed:m_GenericData {X} BaseValue',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {'path': path, 'value': 2.0},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('Conflicting'));
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      // Both pending entries survive so the user can resolve the conflict.
      expect(notifier.state.pendingEdits.length, 2);
    },
  );

  test(
    'saveAllPending refuses a typed edit to a glossary quest CurrentState path',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      const questStatePath = [
        'QuestDataByClass',
        '{/Script/Angelscript.Quest_CreaturesGlossary_Wolf_WolfUnlock}',
        'CurrentState',
      ];
      notifier.setPendingGlossarySegment(
        const GlossarySegmentEdit(
          documentClass: '/Script/Angelscript.Document_Glossary_Wolf',
          segmentClass:
              '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Unlock',
          unlocked: true,
          questStatePath: questStatePath,
        ),
      );
      notifier.setPendingEdit(
        'typed:wolf-current-state',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': questStatePath,
                'value': 'EQuestState::Succeeded',
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('same quest CurrentState'));
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      expect(notifier.state.pendingEdits, hasLength(2));
    },
  );

  test('saveAllPending refuses a container edit to a glossary quest CurrentState '
      'path spelled another way', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    // The index the editor wrote as [4]; the core reads both spellings as the
    // same segment, so the pair has to be refused here rather than split into
    // two writes where the later one silently wins.
    notifier.setPendingGlossarySegment(
      const GlossarySegmentEdit(
        documentClass: '/Script/Angelscript.Document_Glossary_Wolf',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Unlock',
        unlocked: true,
        questStatePath: [
          'QuestDataByClass',
          '{/Script/Angelscript.Quest_CreaturesGlossary_Wolf_WolfUnlock}',
          'SubQuests',
          '[4]',
          'CurrentState',
        ],
      ),
    );
    notifier.setPendingEdit(
      'typed:wolf-current-state',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.arrayRemove',
            'value': {
              'path': [
                'QuestDataByClass',
                '{/Script/Angelscript.Quest_CreaturesGlossary_Wolf_WolfUnlock}',
                'SubQuests',
                '[04]',
                'CurrentState',
              ],
              'index': 0,
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('same quest CurrentState'));
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
    expect(notifier.state.pendingEdits, hasLength(2));
  });

  test('saveAllPending refuses a typed value edit inside Hero MemorizedEvents '
      'alongside a glossary segment change', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingGlossarySegment(
      const GlossarySegmentEdit(
        documentClass: '/Script/Angelscript.Document_Glossary_Wolf',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Unlock',
        unlocked: false,
      ),
    );
    notifier.setPendingEdit(
      'typed:hero-memory-time',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': [
                'LongTermMemoryByGlobalId',
                '{Hero}',
                'MemorizedEvents',
                '[17]',
                'Time',
                'TotalSeconds',
              ],
              'value': 42.0,
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('Hero MemorizedEvents'));
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
    expect(notifier.state.pendingEdits, hasLength(2));
  });

  test('saveAllPending refuses a typed array edit on Hero MemorizedEvents '
      'alongside a glossary segment change', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingGlossarySegment(
      const GlossarySegmentEdit(
        documentClass: '/Script/Angelscript.Document_Glossary_Wolf',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Entry2',
        unlocked: true,
      ),
    );
    notifier.setPendingEdit(
      'typed:hero-memory-remove',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.arrayRemove',
            'value': {
              'path': ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
              'index': 17,
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('Hero MemorizedEvents'));
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
    expect(notifier.state.pendingEdits, hasLength(2));
  });

  test('saveAllPending refuses a typed relationship value edit for the same '
      'NPC as a structured override', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.friend);
    notifier.setPendingEdit(
      'typed:asghan-relationship',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': [
                'm_GenericData',
                '{Relationship}',
                'RelationshipByGlobalId',
                '{Asghan-1}',
                'ActivePersonalRelationshipModifiers',
                '[0]',
                'Relationship',
              ],
              'value': 'ERelationship::Enemy',
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('same NPC relationship entry'));
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
    expect(notifier.state.pendingEdits, hasLength(2));
  });

  test('saveAllPending refuses a typed modifier removal for the same NPC as '
      'a structured relationship override', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.neutral);
    notifier.setPendingEdit(
      'typed:asghan-modifier-remove',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.arrayRemove',
            'value': {
              'path': [
                'RelationshipByGlobalId',
                '{ASGHAN-1}',
                'ActivePersonalRelationshipModifiers',
              ],
              'index': 0,
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isFalse);
    expect(notifier.state.error, contains('same NPC relationship entry'));
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
  });

  test('saveAllPending permits an All-data relationship edit for a different '
      'NPC', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.enemy);
    notifier.setPendingEdit(
      'typed:buster-relationship',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': [
                'RelationshipByGlobalId',
                '{Buster-1}',
                'ActivePersonalRelationshipModifiers',
                '[0]',
                'Relationship',
              ],
              'value': 'ERelationship::Friend',
            },
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isTrue);
    final writes = core.requests
        .where((request) => request.command == 'write_save')
        .toList();
    // Both intents are saved together. The requirement is the ORDER: the
    // index-addressed All-data edit resolves against the pre-splice layout,
    // so it must precede the structural relationship edit for the other NPC.
    expect(writes, hasLength(1));
    expect(
      (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
      ['private.typed.setValue', 'private.npc.setRelationship'],
    );
  });

  test(
    'saveAllPending orders a typed edit ahead of a splicing edit in one write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'inventory',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.removeItem',
              'value': {'path': '/Script/Angelscript.ItMi_Orenugget'},
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'typed:m_GenericData {X} BaseValue',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': ['m_GenericData', '{X}', 'BaseValue'],
                'value': 1.0,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      // No "must be saved on its own" guard fires anymore — the ordering
      // inside one write_save replaces it.
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // One write; the fixed typed edit LEADS so it resolves against the layout
      // the user saw, and the splicing removeItem follows it.
      expect(writes, hasLength(1));
      expect(
        (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue', 'private.inventory.removeItem'],
      );
      // All pending cleared after success.
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test('saveAllPending orders a mixed batch fixed-first, splices after — '
      'with the backup on the one write', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'npc.revive:Lizard-1',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.npc.revive',
            'value': {'id': 'Lizard-1'},
          },
        ],
      ),
    );
    notifier.setPendingEdit(
      'inventory',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.inventory.addItem',
            'value': {'path': '/Script/Angelscript.ItMi_Orenugget', 'count': 1},
          },
        ],
      ),
    );
    notifier.setPendingEdit(
      'attr:Health',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.player.setAttribute',
            'value': {'id': 'Health', 'baseValue': 77.0, 'currentValue': 66.0},
          },
        ],
      ),
    );

    final ok = await notifier.saveAllPending();

    expect(ok, isTrue);
    expect(notifier.state.error, isNull);
    final writes = core.requests
        .where((r) => r.command == 'write_save')
        .toList();
    // All three ride ONE write, in the order that keeps every intent intact:
    // the fixed setAttribute first (so a splice cannot retarget it), then the
    // splicing addItem and revive.
    expect(writes, hasLength(1));
    expect(
      (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
      [
        'private.player.setAttribute',
        'private.inventory.addItem',
        'private.npc.revive',
      ],
    );

    // Backup-once: exactly one write carries backup:true — one pristine
    // snapshot per Save.
    final backupTrue = writes.where((w) => w.payload['backup'] == true);
    expect(backupTrue, hasLength(1));
    expect(writes.first.payload['backup'], isTrue);
    expect(notifier.state.pendingEdits, isEmpty);
  });

  test(
    'saveAllPending batches two distinct splicing edits into one ordered write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'npc.revive:Lizard-1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'Lizard-1'},
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'knowledge',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.knowledge.addCharacter',
              'value': {'value': 'Diego'},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Neither splice is addressed by an index the other could shift, so both
      // ride one write — in their stable pending-key order.
      expect(writes, hasLength(1));
      expect(
        (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.knowledge.addCharacter', 'private.npc.revive'],
      );
      // Backup-once: the single write takes it.
      expect(writes.single.payload['backup'], isTrue);
    },
  );

  test(
    'saveAllPending orders glossary adds before removals without reordering other splices',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      const document = '/Script/Angelscript.Document_Glossary_Wolf';
      notifier.setPendingEdit(
        'a-glossary-remove',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.glossary.setSegment',
              'value': {
                'documentClass': document,
                'segmentClass':
                    '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Intro',
                'unlocked': false,
              },
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'b-inventory-add',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.addItem',
              'value': {
                'path': '/Script/Angelscript.ItMi_Orenugget',
                'count': 1,
              },
            },
          ],
        ),
      );
      notifier.setPendingNpcRevive('Lizard-1');
      notifier.setPendingEdit(
        'z-glossary-add',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.glossary.setSegment',
              'value': {
                'documentClass': document,
                'segmentClass':
                    '/Script/Angelscript.DocumentSegment_Glossary_Wolf_Dead',
                'unlocked': true,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((request) => request.command == 'write_save')
          .toList();
      // The four splices ride ONE write, so the order INSIDE its payload is now
      // the only thing keeping the glossary add ahead of the removal (an add
      // needs a surviving SegmentUnlocked event as its byte template) and the
      // other splices in their original relative position.
      expect(writes, hasLength(1));
      final edits = (writes.single.payload['edits'] as List).cast<Map>();
      expect(edits.map((edit) => edit['path']), [
        'private.glossary.setSegment',
        'private.inventory.addItem',
        'private.npc.revive',
        'private.glossary.setSegment',
      ]);
      expect((edits.first['value'] as Map)['unlocked'], isTrue);
      expect((edits.last['value'] as Map)['unlocked'], isFalse);
    },
  );

  test(
    'saveAllPending sets syncPersistentDataList exactly once, on the backup write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
          ],
          syncPersistentDataList: true,
        ),
      );
      notifier.setPendingEdit(
        'npc.revive:Lizard-1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'Lizard-1'},
            },
          ],
        ),
      );

      await notifier.saveAllPending();

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Both edits ride one write; the flag is set exactly ONCE and lands on
      // the write that also takes the backup, so the PersistentDataList.sav
      // companion is only ever updated beside a restorable snapshot.
      final synced = writes.where(
        (w) => w.payload['syncPersistentDataList'] == true,
      );
      expect(synced, hasLength(1));
      expect(synced.single.payload['backup'], isTrue);
      expect(
        (synced.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['public.m_PlayerSaveName', 'private.npc.revive'],
      );
    },
  );

  test(
    'saveAllPending refuses an ActiveEffects Def edit queued with a skill edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // A raw All-Data setValue retargeting an ActiveEffects EffectSpec/Def...
      notifier.setPendingEdit(
        'typed:def',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'ActiveEffectsByGlobalId',
                  '{Hero}',
                  'ActiveEffects',
                  '[0]',
                  'EffectSpec',
                  'Def',
                ],
                'value': '/Script/Angelscript.Default__GE_Skill_Sneak',
              },
            },
          ],
        ),
      );
      // ...plus a skill edit (which may splice the array): cannot be sequenced.
      notifier.setPendingEdit(
        'skills',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.skills.set',
              'value': {
                'actor': 'Hero',
                'base': 'Melee_OneHanded',
                'tier': 'Master',
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();
      // Refused: no write at all, and an explanatory error is surfaced.
      expect(ok, isFalse);
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      expect(notifier.state.error, contains('EffectSpec'));
    },
  );

  test(
    'saveAllPending allows a hero skill edit with an NPC ActiveEffects Def edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // A Def edit on a DIFFERENT actor's ActiveEffects array...
      notifier.setPendingEdit(
        'typed:npcdef',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'ActiveEffectsByGlobalId',
                  '{Lizard-1}',
                  'ActiveEffects',
                  '[0]',
                  'EffectSpec',
                  'Def',
                ],
                'value': '/Script/Angelscript.Default__GE_Skill_Sneak',
              },
            },
          ],
        ),
      );
      // ...and a HERO skill edit: separate arrays, so no conflict.
      notifier.setPendingEdit(
        'skills',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.skills.set',
              'value': {
                'actor': 'Hero',
                'base': 'Melee_OneHanded',
                'tier': 'Master',
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();
      expect(ok, isTrue);
      // Both saved. The packing rule only splits an ordinal-addressed edit that
      // comes AFTER an ordinal-invalidating one; here the Def edit's `[0]`
      // carries the ordinal and the skill edit (the invalidating producer) is
      // ordered behind it, so nothing splits — they share ONE write, Def first.
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(1));
      expect(
        (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue', 'private.skills.set'],
      );
    },
  );

  test(
    'saveAllPending treats an actor-less skill edit as Hero for the Def conflict',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // A hero ActiveEffects Def edit...
      notifier.setPendingEdit(
        'typed:def',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'ActiveEffectsByGlobalId',
                  '{Hero}',
                  'ActiveEffects',
                  '[0]',
                  'EffectSpec',
                  'Def',
                ],
                'value': '/Script/Angelscript.Default__GE_Skill_Sneak',
              },
            },
          ],
        ),
      );
      // ...and a skill edit that OMITS actor (the core defaults it to Hero).
      notifier.setPendingEdit(
        'skills',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.skills.set',
              'value': {'base': 'Melee_OneHanded', 'tier': 'Master'},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();
      // Still a same-actor (Hero) collision → refused.
      expect(ok, isFalse);
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
    },
  );

  test(
    'saveAllPending keeps an ActiveEffects Def edit in the batch without a skill edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // No skill edit is queued, so there is no conflict: the Def edit stays in
      // the single fixed-batch write (unchanged behaviour).
      notifier.setPendingEdit(
        'typed:def',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'ActiveEffectsByGlobalId',
                  '{Hero}',
                  'ActiveEffects',
                  '[0]',
                  'EffectSpec',
                  'Def',
                ],
                'value': '/Script/Angelscript.Default__GE_Skill_Sneak',
              },
            },
          ],
        ),
      );

      await notifier.saveAllPending();

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(1));
    },
  );

  test('same-path reinspection invalidates shared in-flight reads', () async {
    Map<String, Object?> heroHit(String leaf) => {
      'path': [
        'm_GenericData',
        '{CharacterStates}',
        'AnyCharacterType',
        'AttributesByGlobalId',
        '{Hero}',
        'AttributeSetsByClass',
        '{/Script/G1R.AttributeSet_Health}',
        'Attributes',
        '{MaxHealth}',
        leaf,
      ],
      'display': '…',
      'type': 'FloatProperty',
      'value': '64',
      'editable': true,
    };
    final pageTwoGate = Completer<void>();
    final core = _RecordingCoreService(
      typedSearchPages: [
        {
          'query': 'AttributesByGlobalId {Hero}',
          'offset': 0,
          'limit': 1000,
          'total': 2,
          'count': 1,
          'results': [heroHit('BaseValue')],
        },
        {
          'query': 'AttributesByGlobalId {Hero}',
          'offset': 1,
          'limit': 1000,
          'total': 2,
          'count': 1,
          'results': [heroHit('CurrentValue')],
        },
        {
          'query': 'AttributesByGlobalId {Hero}',
          'offset': 0,
          'limit': 1000,
          'total': 1,
          'count': 1,
          'results': [heroHit('BaseValue')],
        },
      ],
    );
    late final EditorNotifier notifier;
    Future<void>? reinspection;
    core.onTypedSearchCall = (call) {
      if (call == 0) {
        reinspection = notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      }
    };
    core.waitTypedSearchCall = (call) =>
        call == 1 ? pageTwoGate.future : Future<void>.value();
    notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    final originalInspection = notifier.state.inspection;

    final stale = notifier.loadHeroAttributes();
    while (identical(notifier.state.inspection, originalInspection)) {
      await pumpEventQueue();
    }
    final fresh = notifier.loadHeroAttributes();

    expect(identical(fresh, stale), isFalse);
    pageTwoGate.complete();
    await Future.wait([stale, fresh, reinspection!]);
    expect(
      core.requests.where(
        (request) => request.command == 'search_typed_properties',
      ),
      hasLength(3),
    );
  });

  // ---------------------------------------------------------------------------
  // The core's SAME-TARGET rule is order-independent: a structured operation
  // rewrites its target wholesale, so a raw typed edit addressing what it
  // rewrites is refused in the same write whichever way round the two come.
  // The packer must SPLIT those pairs into sequential sub-writes (which is how
  // they ran before batching existed) instead of building a write the core
  // rejects — a rejection fails the whole Save with nothing committed.
  // ---------------------------------------------------------------------------

  test(
    'saveAllPending splits an All-Data MemorizedEvents edit from an NPC revive',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // A plain value edit inside an NPC's MemorizedEvents (not a structural
      // array op, so the memory-event guard above does not refuse it)...
      notifier.setPendingEdit(
        'typed:lizard-memory-time',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'LongTermMemoryByGlobalId',
                  '{Lizard-1}',
                  'MemorizedEvents',
                  '[2]',
                  'Time',
                  'TotalSeconds',
                ],
                'value': 12.0,
              },
            },
          ],
        ),
      );
      // ...plus the revive, which strips memory events wholesale.
      notifier.setPendingEdit(
        'npc.revive:Lizard-1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'Lizard-1'},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // TWO writes: sequential, so the typed edit lands and the revive then
      // re-reads the file and strips events from what is on disk.
      expect(writes, hasLength(2));
      expect(
        (writes[0].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue'],
      );
      expect(
        (writes[1].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.npc.revive'],
      );
      // Backup-once still holds, on the first write.
      expect(writes.where((w) => w.payload['backup'] == true), hasLength(1));
      expect(writes.first.payload['backup'], isTrue);
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test(
    'saveAllPending splits a knowledge edit from a typed edit in the SAME entry',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'knowledge',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.knowledge.setEntry',
              'value': {
                'character': 'Diego',
                'entry': 'Info_Whatslife',
                'present': true,
              },
            },
          ],
        ),
      );
      // An All-data edit inside the very entry that setEntry rewrites.
      notifier.setPendingEdit(
        'typed:diego-knowledge',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'CharacterKnowledgeByUniqueName',
                  '{Diego}',
                  'Knowledge',
                  'm_Size',
                ],
                'value': 3,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(2));
      expect(
        (writes[0].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue'],
      );
      expect(
        (writes[1].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.knowledge.setEntry'],
      );
    },
  );

  test(
    'saveAllPending keeps a knowledge edit and a DIFFERENT entry in one write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'knowledge',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.knowledge.setEntry',
              'value': {
                'character': 'Diego',
                'entry': 'Info_Whatslife',
                'present': true,
              },
            },
          ],
        ),
      );
      // Another character's entry is a different map value; every applier
      // re-resolves its target by key, so the two do not collide.
      notifier.setPendingEdit(
        'typed:xardas-knowledge',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'CharacterKnowledgeByUniqueName',
                  '{Xardas}',
                  'Knowledge',
                  'm_Size',
                ],
                'value': 3,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(1));
      expect(
        (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue', 'private.knowledge.setEntry'],
      );
    },
  );

  test(
    'saveAllPending splits a non-Def ActiveEffects edit from a same-actor skill edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // A leaf of the hero's ActiveEffects that is NOT EffectSpec/Def, so the
      // same-actor Def refusal does not fire — but the core still rejects the
      // pair in one write, because a skill edit rewrites that actor's effect
      // elements wholesale.
      notifier.setPendingEdit(
        'typed:effect-level',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'ActiveEffectsByGlobalId',
                  '{Hero}',
                  'ActiveEffects',
                  '[0]',
                  'EffectSpec',
                  'Level',
                ],
                'value': 3.0,
              },
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'skills',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.skills.set',
              'value': {
                'actor': 'Hero',
                'base': 'Melee_OneHanded',
                'tier': 'Master',
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      // Not refused — split. The old Def-only check would have let this ride
      // one write, which the core now rejects outright.
      expect(ok, isTrue);
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(2));
      expect(
        (writes[0].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue'],
      );
      expect(
        (writes[1].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.skills.set'],
      );
    },
  );

  test(
    'saveAllPending splits the slot repair from an m_Id below a slot payload',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // An m_Id that is not the slot's own leaf: narrower than the repair's
      // refusal guard, so it is not refused — but the core counts it as a
      // conflict, so it has to be split.
      notifier.setPendingEdit(
        'typed:payload-id',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'm_SavedPlayers',
                  '[0]',
                  'm_Inventory',
                  'm_Slots',
                  '[3]',
                  'm_Payload',
                  'm_Id',
                ],
                'value': 4,
              },
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'inventory:repair',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.repairSlots',
              'value': <String, Object?>{},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(2));
      expect(
        (writes[0].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue'],
      );
      expect(
        (writes[1].payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.inventory.repairSlots'],
      );
    },
  );

  test(
    'saveAllPending keeps the slot repair with a non-id slot edit in one write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // The repair only rewrites ids, so anything else inside a slot survives
      // it untouched — the new split must not over-fire here.
      notifier.setPendingEdit(
        'typed:slot-count',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [
                  'm_SavedPlayers',
                  '[0]',
                  'm_Inventory',
                  'm_Slots',
                  '[3]',
                  'm_SlotData',
                  'm_ItemCount',
                ],
                'value': 7,
              },
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'inventory:repair',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.repairSlots',
              'value': <String, Object?>{},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(1));
      // The repair still runs last inside that write.
      expect(
        (writes.single.payload['edits'] as List).map((e) => (e as Map)['path']),
        ['private.typed.setValue', 'private.inventory.repairSlots'],
      );
    },
  );

  group('the same-target predicate the packer splits on', () {
    Map<String, Object?> typedEdit(List<String> path) => {
      'path': 'private.typed.setValue',
      'value': {'path': path, 'value': 1},
    };
    Map<String, Object?> addItem(String? actorId) => {
      'path': 'private.inventory.addItem',
      'value': {
        'path': '/Script/Angelscript.ItMi_Orenugget',
        'count': 1,
        'actorId': ?actorId,
      },
    };
    const playerSlot = [
      'm_SavedPlayers',
      '[0]',
      'm_Inventory',
      'm_Slots',
      '[3]',
      'm_SlotData',
      'm_ItemCount',
    ];
    List<String> npcSlot(String id) => [
      'm_GenericData',
      '{CharacterStates}',
      'NPCCharacters',
      'InventoryByGlobalId',
      '{$id}',
      'InventoryItems',
      'm_Values',
      'Items',
      '[6]',
      'm_Slots',
      '[3]',
      'm_SlotData',
      'm_ItemCount',
    ];

    test('an inventory add only claims ITS actor\'s slots', () {
      // The core allows an add for one actor beside a raw slot edit for
      // another, so the packer must not split them. (Today saveAllPending
      // refuses that combination outright, one refusal earlier — this pins the
      // packing rule itself, which is what the core enforces.)
      expect(
        editsRewriteSameTarget(addItem(null), typedEdit(playerSlot)),
        isTrue,
      );
      expect(
        editsRewriteSameTarget(addItem(null), typedEdit(npcSlot('Lizard-1'))),
        isFalse,
      );
      expect(
        editsRewriteSameTarget(
          addItem('Lizard-1'),
          typedEdit(npcSlot('Lizard-1')),
        ),
        isTrue,
      );
      expect(
        editsRewriteSameTarget(
          addItem('Lizard-1'),
          typedEdit(npcSlot('Lizard-2')),
        ),
        isFalse,
      );
      expect(
        editsRewriteSameTarget(addItem('Lizard-1'), typedEdit(playerSlot)),
        isFalse,
      );
    });

    test('an actor key matches trimmed and case-insensitively', () {
      expect(
        editsRewriteSameTarget(
          addItem('lizard-1'),
          typedEdit(npcSlot(' LIZARD-1 ')),
        ),
        isTrue,
      );
    });

    test('it holds in BOTH directions', () {
      final revive = {
        'path': 'private.npc.revive',
        'value': {'id': 'Lizard-1'},
      };
      final typed = typedEdit([
        'LongTermMemoryByGlobalId',
        '{Lizard-1}',
        'MemorizedEvents',
      ]);
      expect(editsRewriteSameTarget(revive, typed), isTrue);
      expect(editsRewriteSameTarget(typed, revive), isTrue);
    });

    test('a glossary segment claims the quest CurrentState it rewrites', () {
      Map<String, Object?> glossary(List<String>? questStatePath) => {
        'path': 'private.glossary.setSegment',
        'value': {
          'documentClass': '/Script/Angelscript.Document_Glossary_Meatbug',
          'segmentClass':
              '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Entry2',
          'unlocked': true,
          'questStatePath': ?questStatePath,
        },
      };
      const questState = [
        'QuestDataByClass',
        '{/Script/Angelscript.Quest_OldCamp_SLEEPER}',
        'CurrentState',
      ];

      // The core rewrites that CurrentState itself, so the two cannot share a
      // write — in either order.
      expect(
        editsRewriteSameTarget(glossary(questState), typedEdit(questState)),
        isTrue,
      );
      expect(
        editsRewriteSameTarget(typedEdit(questState), glossary(questState)),
        isTrue,
      );

      // Sending no quest path does not make it someone else's business: the core
      // then derives the same leaf from the document and segment. Which leaf that
      // is cannot be known here, so any quest's CurrentState is claimed.
      expect(
        editsRewriteSameTarget(glossary(null), typedEdit(questState)),
        isTrue,
      );
      expect(
        editsRewriteSameTarget(
          glossary(null),
          typedEdit([
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_Something_Else}',
            'CurrentState',
          ]),
        ),
        isTrue,
      );

      // Another field of the same entry, and a CurrentState that is not a
      // quest's, stay packable.
      expect(
        editsRewriteSameTarget(
          glossary(questState),
          typedEdit([
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_OldCamp_SLEEPER}',
            'm_Comment',
          ]),
        ),
        isFalse,
      );
      expect(
        editsRewriteSameTarget(
          glossary(questState),
          typedEdit(['m_GenericData', '{Dialogue}', 'CurrentState']),
        ),
        isFalse,
      );
    });

    test('the packer really splits such a pair into two writes', () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:	mp\saves');
      await notifier.inspect(r'C:	mp\saves\G1R-001.sav');

      Map<String, Object?> entry(
        String character,
        String name,
        bool present,
      ) => {
        'path': 'private.knowledge.setEntry',
        'value': {'character': character, 'entry': name, 'present': present},
      };
      // Two registry entries the core folds into one target. It refuses the
      // pair, so both have to reach it in writes of their own.
      notifier.setPendingEdit(
        'knowledge:a',
        PendingSaveEdit(edits: [entry('Diego', 'Info_Ore ', false)]),
      );
      notifier.setPendingEdit(
        'knowledge:b',
        PendingSaveEdit(edits: [entry('diego', 'Info_Ore', true)]),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((request) => request.command == 'write_save')
          .toList();
      expect(writes, hasLength(2));
      for (final write in writes) {
        expect((write.payload['edits'] as List), hasLength(1));
      }
    });

    test(
      'an id-addressed edit is written before the write that renumbers ids',
      () async {
        final core = _RecordingCoreService();
        final notifier = EditorNotifier(core, saveDir: r'C:	mp\saves');
        await notifier.inspect(r'C:	mp\saves\G1R-001.sav');

        // A raw write to a slot's m_Id renumbers what the count edit is addressed
        // by. Two writes would not help — the second would resolve the id this one
        // moved — so they share one write with the addressed edit first, which is
        // the order the core accepts.
        notifier.setPendingEdit(
          'a-slot-id',
          const PendingSaveEdit(
            edits: [
              {
                'path': 'private.typed.setValue',
                'value': {
                  'path': [
                    'm_Inventory',
                    'm_Containers',
                    '[0]',
                    'm_Slots',
                    '[3]',
                    'm_Id',
                  ],
                  'value': 9,
                },
              },
            ],
          ),
        );
        notifier.setPendingEdit(
          'z-count',
          const PendingSaveEdit(
            edits: [
              {
                'path': 'private.inventory.setItemCount',
                'value': {
                  'path': '/Script/Angelscript.ItMi_Orenugget',
                  'count': 5,
                },
              },
            ],
          ),
        );

        final ok = await notifier.saveAllPending();

        expect(ok, isTrue);
        final writes = core.requests
            .where((request) => request.command == 'write_save')
            .toList();
        expect(writes, hasLength(1));
        final edits = (writes.single.payload['edits'] as List)
            .cast<Map<String, Object?>>();
        expect(edits.map((edit) => edit['path']), [
          'private.inventory.setItemCount',
          'private.typed.setValue',
        ]);
      },
    );

    test('a skill edit sent with an empty actor is the hero everywhere', () {
      final emptyActor = {
        'path': 'private.skills.set',
        'value': {'actor': '', 'base': 'Ranged_Bow', 'tier': 'Trained'},
      };
      final heroDef = typedEdit([
        'ActiveEffectsByGlobalId',
        '{Hero}',
        'ActiveEffects',
        '[2]',
        'EffectSpec',
        'Def',
      ]);

      // Both rules read the actor the same way, so the packer splits this pair
      // exactly where the core refuses it.
      expect(editsRewriteSameTarget(emptyActor, heroDef), isTrue);
      expect(
        structuredEditsShareATarget(emptyActor, {
          'path': 'private.skills.set',
          'value': {'actor': 'Hero', 'base': 'Ranged_Bow', 'tier': 'Untrained'},
        }),
        isTrue,
      );
    });

    test('two structured edits for one target land in separate writes', () {
      Map<String, Object?> skill(String actor, String base, String tier) => {
        'path': 'private.skills.set',
        'value': {'actor': actor, 'base': base, 'tier': tier},
      };

      // The core refuses this pair outright, so the packer has to split it or
      // the whole Save fails and takes every unrelated edit with it. The parts
      // of the target are folded as the core folds them.
      expect(
        structuredEditsShareATarget(
          skill('Hero', 'Ranged_Bow', 'Trained'),
          skill(' hero ', 'ranged_bow', 'Untrained'),
        ),
        isTrue,
      );

      // The core reads an ABSENT or empty actor as the hero, and it decides that
      // before folding, so a blank of spaces stays an actor of its own.
      expect(
        structuredEditsShareATarget(skill('Hero', 'Ranged_Bow', 'Trained'), {
          'path': 'private.skills.set',
          'value': {'base': 'Ranged_Bow', 'tier': 'Untrained'},
        }),
        isTrue,
      );
      expect(
        structuredEditsShareATarget(
          skill('Hero', 'Ranged_Bow', 'Trained'),
          skill('', 'Ranged_Bow', 'Untrained'),
        ),
        isTrue,
      );
      expect(
        structuredEditsShareATarget(
          skill('Hero', 'Ranged_Bow', 'Trained'),
          skill(' ', 'Ranged_Bow', 'Untrained'),
        ),
        isFalse,
      );

      // Two skills of one character, and one skill of two characters, are two
      // targets — batching those is the point.
      expect(
        structuredEditsShareATarget(
          skill('Hero', 'Ranged_Bow', 'Trained'),
          skill('Hero', 'Ranged_Crossbow', 'Trained'),
        ),
        isFalse,
      );
      expect(
        structuredEditsShareATarget(
          skill('Hero', 'Ranged_Bow', 'Trained'),
          skill('Diego', 'Ranged_Bow', 'Trained'),
        ),
        isFalse,
      );

      // The core folds ASCII case only, so two names that differ outside ASCII
      // stay two targets here as well.
      expect(
        structuredEditsShareATarget(
          skill('HÄro', 'Ranged_Bow', 'Trained'),
          skill('häro', 'Ranged_Bow', 'Untrained'),
        ),
        isFalse,
      );
      expect(
        structuredEditsShareATarget(
          skill('Häro', 'Ranged_Bow', 'Trained'),
          skill('häro', 'Ranged_Bow', 'Untrained'),
        ),
        isTrue,
      );

      // A glossary segment is keyed by the two asset names, as the core keys it.
      Map<String, Object?> segment(String document, String seg) => {
        'path': 'private.glossary.setSegment',
        'value': {
          'documentClass': document,
          'segmentClass': seg,
          'unlocked': true,
        },
      };
      expect(
        structuredEditsShareATarget(
          segment(
            '/Script/Angelscript.Document_Glossary_Meatbug',
            '/Script/Angelscript.DocumentSegment_Meatbug_Entry2',
          ),
          segment(
            '/Script/Angelscript.Document_Glossary_Meatbug',
            '/Script/Angelscript.DocumentSegment_Meatbug_Unlock',
          ),
        ),
        isFalse,
      );

      // An operation that adds to what is there has no such target at all.
      expect(
        structuredEditsShareATarget(
          {
            'path': 'private.inventory.addItem',
            'value': {'path': '/Script/Angelscript.ItFo_Cheese', 'count': 1},
          },
          {
            'path': 'private.inventory.addItem',
            'value': {'path': '/Script/Angelscript.ItFo_Cheese', 'count': 1},
          },
        ),
        isFalse,
      );
    });

    test('addressing a whole map collides with every entry in it', () {
      final setEntry = {
        'path': 'private.knowledge.setEntry',
        'value': {'character': 'Diego', 'entry': 'Info_X', 'present': true},
      };
      expect(
        structuredEditRewrites(setEntry, const [
          'CharacterKnowledgeByUniqueName',
        ]),
        isTrue,
      );
      expect(
        structuredEditRewrites(setEntry, const [
          'CharacterKnowledgeByUniqueName',
          '{Diego}',
        ]),
        isTrue,
      );
      expect(
        structuredEditRewrites(setEntry, const [
          'CharacterKnowledgeByUniqueName',
          '{Xardas}',
        ]),
        isFalse,
      );
    });

    test('a skill edit with no actor defends the hero\'s effects', () {
      final skills = {
        'path': 'private.skills.set',
        'value': {'base': 'Melee_OneHanded', 'tier': 'Master'},
      };
      expect(
        structuredEditRewrites(skills, const [
          'ActiveEffectsByGlobalId',
          '{Hero}',
          'ActiveEffects',
          '[0]',
          'EffectSpec',
          'Level',
        ]),
        isTrue,
      );
      expect(
        structuredEditRewrites(skills, const [
          'ActiveEffectsByGlobalId',
          '{Lizard-1}',
          'ActiveEffects',
          '[0]',
          'EffectSpec',
          'Level',
        ]),
        isFalse,
      );
    });

    test('two raw typed edits never count as a same-target pair', () {
      // Only a STRUCTURED operation rewrites a target wholesale; two typed
      // edits are ordered by the ordinal rule instead.
      expect(
        editsRewriteSameTarget(
          typedEdit(playerSlot),
          typedEdit(const [
            'm_SavedPlayers',
            '[0]',
            'm_Inventory',
            'm_Slots',
            '[3]',
            'm_Id',
          ]),
        ),
        isFalse,
      );
    });
  });

  // ---------------------------------------------------------------------------
  // Bug #1: a fresh inspection must reset the selected actor to the player so a
  // stale NPC GlobalId from the previous save can't drive the actor-aware tabs.
  // ---------------------------------------------------------------------------
  test(
    'inspecting a new save resets the selected actor to the player',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Select an NPC against the first save.
      notifier.selectActor(
        const Actor.npc(id: 'Lizard-1', name: 'Lizard', uniqueName: 'Lizard'),
      );
      expect(notifier.state.selectedActor.isPlayer, isFalse);

      // Switch to a DIFFERENT save: the NPC id belongs to the old file, so the
      // selection must fall back to the always-valid player.
      await notifier.inspect(r'C:\tmp\saves\G1R-002.sav');

      expect(notifier.state.selectedActor.isPlayer, isTrue);
    },
  );

  // Codex follow-up: a SAME-save refresh (after a save/reset) must NOT reset the
  // selection — the NPC id is still valid, so NPC editing shouldn't jump to Player.
  test('same-save refresh preserves the selected NPC', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.selectActor(
      const Actor.npc(id: 'Lizard-1', name: 'Lizard', uniqueName: 'Lizard'),
    );
    expect(notifier.state.selectedActor.isPlayer, isFalse);

    // Re-inspect the SAME save (what saveAllPending()/refresh() do).
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    expect(notifier.state.selectedActor.isPlayer, isFalse);
    expect(notifier.state.selectedActor.id, 'Lizard-1');
  });

  // ---------------------------------------------------------------------------
  // Bug #2: a mixed [npc.revive + npc Health setValue for the SAME id] must run
  // the fixed (Health) batch BEFORE the revive splice, so revive's HP wins.
  // ---------------------------------------------------------------------------
  test(
    'saveAllPending runs the fixed batch before a conflicting revive splice',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'npc.revive:Lizard-1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'Lizard-1'},
            },
          ],
        ),
      );
      notifier.setPendingEdit(
        'npc.attributes:Lizard-1',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': ['…', 'Lizard-1', 'Health', 'CurrentValue'],
                'value': 42.0,
              },
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();
      expect(ok, isTrue);

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Both edits ride one write; the core applies a batch sequentially, so
      // position in the payload is what orders them now.
      expect(writes, hasLength(1));
      final paths = (writes.single.payload['edits'] as List)
          .map((e) => (e as Map)['path'])
          .toList();
      // The fixed Health edit is applied BEFORE the revive, so revive's HP
      // (the last write to the NPC's HP) is final on disk.
      expect(
        paths.indexOf('private.typed.setValue'),
        lessThan(paths.indexOf('private.npc.revive')),
      );
      expect(paths, ['private.typed.setValue', 'private.npc.revive']);
    },
  );

  // ---------------------------------------------------------------------------
  // Bug #3: when a synced public edit and a splicing edit are both pending, the
  // synced (syncPersistentDataList) write must be the backup-taking write, so
  // the PersistentDataList.sav companion is updated WITH a restorable backup.
  // ---------------------------------------------------------------------------
  test(
    'saveAllPending makes the syncPersistentDataList write the backup write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
          ],
          syncPersistentDataList: true,
        ),
      );
      notifier.setPendingEdit(
        'inventory',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.inventory.addItem',
              'value': {
                'path': '/Script/Angelscript.ItMi_Orenugget',
                'count': 1,
              },
            },
          ],
        ),
      );

      await notifier.saveAllPending();

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // The flag is set exactly once, on the write that also takes the backup.
      expect(
        writes.where((w) => w.payload['syncPersistentDataList'] == true),
        hasLength(1),
      );
      final synced = writes.singleWhere(
        (w) => w.payload['syncPersistentDataList'] == true,
      );
      // The synced write also carries backup:true (companion is backed up).
      expect(synced.payload['backup'], isTrue);
      // Backup-once still holds: exactly one write takes a backup.
      expect(writes.where((w) => w.payload['backup'] == true), hasLength(1));
    },
  );

  // ---------------------------------------------------------------------------
  // Bug #4: loadAllNpcActors must PAGE through private.npc.list (core clamps the
  // limit to 1000) so every NPC reaches the client cache, not just the first
  // 1000.
  // ---------------------------------------------------------------------------
  test('loadAllNpcActors pages through the clamped NPC list', () async {
    const total = 1484;
    final core = _PagedNpcCoreService(total: total, pageSize: 1000);
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadAllNpcActors();

    expect(page.error, isNull);
    expect(page.npcs, hasLength(total));
    expect(page.total, total);
    // Two list calls: first 1000, then the remaining 484.
    final listCalls = core.requests
        .where((r) => r.command == 'private.npc.list')
        .toList();
    expect(listCalls, hasLength(2));
    expect(listCalls[0].payload['offset'], 0);
    expect(listCalls[1].payload['offset'], 1000);
  });

  // Cursor (High): loadAllNpcActors must PIN the save path for the whole
  // multi-page fetch so a mid-fetch save switch can't merge pages from two files.
  test(
    'loadAllNpcActors pins the save path across a mid-fetch save switch',
    () async {
      final core = _MidFetchSwitchNpcCoreService(total: 1484, pageSize: 1000);
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      // After the first page returns, switch to a different save mid-fetch.
      // Fire-and-forget: the switch is queued behind the in-flight fetch; pinning
      // must keep page 2 on the save the fetch started against regardless.
      core.onFirstListPage = () {
        // ignore: unawaited_futures
        notifier.inspect(r'C:\tmp\saves\G1R-002.sav');
      };

      final page = await notifier.loadAllNpcActors();

      expect(page.error, isNull);
      expect(page.npcs, hasLength(1484));
      final listCalls = core.requests
          .where((r) => r.command == 'private.npc.list')
          .toList();
      expect(listCalls, hasLength(2));
      // BOTH pages target the save the fetch STARTED against — never the new one.
      expect(listCalls[0].payload['path'], r'C:\tmp\saves\G1R-001.sav');
      expect(listCalls[1].payload['path'], r'C:\tmp\saves\G1R-001.sav');
    },
  );

  // ---------------------------------------------------------------------------
  // Charaktere master list index (Task 10: Player/Hero de-duplication). The
  // save's own "Hero" ACTOR row keys the player's memory events; the pinned
  // Player row represents it, so its GlobalId is stashed for the events wiring.
  // ---------------------------------------------------------------------------

  test('loadAllCharacters stashes the hero actor GlobalId', () async {
    final core = _CharactersListCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    expect(notifier.state.heroGlobalId, isNull);
    expect(notifier.state.heroGlobalIdSettled, isFalse);

    final page = await notifier.loadAllCharacters();

    expect(page.error, isNull);
    expect(page.characters, hasLength(3));
    expect(notifier.state.heroGlobalId, 'Hero');
    // The load completed — the hero id is settled for this save.
    expect(notifier.state.heroGlobalIdSettled, isTrue);
  });

  test(
    'loadAllCharacters leaves the stashed heroGlobalId untouched on an error page',
    () async {
      final core = _CharactersListCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      await notifier.loadAllCharacters();
      expect(notifier.state.heroGlobalId, 'Hero');

      core.failList = true;
      final page = await notifier.loadAllCharacters();

      expect(page.error, isNotNull);
      // A stale value from the same save is still correct — keep it.
      expect(notifier.state.heroGlobalId, 'Hero');
      // The failed attempt still COMPLETED, so the id stays settled.
      expect(notifier.state.heroGlobalIdSettled, isTrue);
    },
  );

  // Cursor (Medium): with the Player selected, the Ereignisse pane spun
  // forever when the index load failed before ever stashing a hero id. A
  // completed attempt — even a failed one — must mark the id settled so the
  // pane can leave the spinner for the empty state.
  test(
    'loadAllCharacters settles the hero id even when the very first load fails',
    () async {
      final core = _CharactersListCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      expect(notifier.state.heroGlobalIdSettled, isFalse);

      core.failList = true;
      final page = await notifier.loadAllCharacters();

      expect(page.error, isNotNull);
      // No id was ever stashed, but the load completed: settled, id null.
      expect(notifier.state.heroGlobalId, isNull);
      expect(notifier.state.heroGlobalIdSettled, isTrue);
    },
  );

  // Cursor (Medium): the hero GlobalId belongs to ONE save. A slot switch must
  // drop it so the player's Ereignisse sub-tab never queries the previous
  // save's id against the new file.
  test('slot switch clears the stashed heroGlobalId', () async {
    final core = _CharactersListCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await notifier.loadAllCharacters();
    expect(notifier.state.heroGlobalId, 'Hero');
    expect(notifier.state.heroGlobalIdSettled, isTrue);

    await notifier.inspect(r'C:\tmp\saves\G1R-002.sav');

    expect(notifier.state.heroGlobalId, isNull);
    // The new save's index has not completed yet — the settled flag resets
    // with the id, so the events pane shows the spinner, not the empty state.
    expect(notifier.state.heroGlobalIdSettled, isFalse);
  });

  // Cursor (Medium): a slow characters.list response must not stash the
  // PREVIOUS save's hero id after the user already switched slots — the stash
  // is pinned to the path the request was issued against.
  test('mid-fetch slot switch discards the stale hero stash', () async {
    final core = _CharactersListCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    // Switch slots while the characters.list call is in flight: _inspect sets
    // selectedPath synchronously, so by the time the list response is parsed
    // the request's path no longer matches the selection.
    core.onListCall = () {
      // ignore: unawaited_futures
      notifier.inspect(r'C:\tmp\saves\G1R-002.sav');
    };
    final page = await notifier.loadAllCharacters();

    expect(page.error, isNull);
    expect(notifier.state.heroGlobalId, isNull);
    // The settled flag is pinned to the same path: the stale completion must
    // not mark the NEW save's index as settled either.
    expect(notifier.state.heroGlobalIdSettled, isFalse);
  });

  test('failed same-save re-inspect keeps pending edits retryable', () async {
    final core = _FailingSecondInspectCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );

    // The re-inspect fails: editors keep showing the drafts (no fresh
    // inspection re-seeded them), so the registry must keep matching them.
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    expect(notifier.state.error, isNotNull);
    expect(notifier.state.pendingEdits.isNotEmpty, isTrue);
  });

  // ---------------------------------------------------------------------------
  // Regression tests for finding 1: central pending-edit lifecycle
  // ---------------------------------------------------------------------------

  test('refresh() clears all pending edits (same slot)', () async {
    // Central clear on refresh prevents widgets from mutating the provider
    // during build (which throws with flutter_riverpod).
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );
    notifier.setPendingEdit(
      'heroStats',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': ['MaxHealth'],
              'value': 99.0,
            },
          },
        ],
      ),
    );
    expect(notifier.state.pendingEdits.length, 2);

    // Toolbar Refresh — same selected path stays selected.
    await notifier.refresh();

    expect(
      notifier.state.pendingEdits,
      isEmpty,
      reason: 'refresh() must clear ALL pending edits',
    );
  });

  test('restoreBackup() clears all pending edits via refresh()', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'Draft'},
        ],
      ),
    );
    expect(notifier.state.pendingEdits.isNotEmpty, isTrue);

    await notifier.restoreBackup(r'C:\tmp\saves\G1R-001.sav.bak.200');

    expect(
      notifier.state.pendingEdits,
      isEmpty,
      reason: 'restoreBackup() must clear pending edits via refresh()',
    );
  });

  test('pendingEditCount on EditorState counts individual edits', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    // One entry with 2 edits and another with 1 edit → count = 3.
    notifier.setPendingEdit(
      'heroStats',
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': ['MaxHealth'],
              'value': 99.0,
            },
          },
          {
            'path': 'private.typed.setValue',
            'value': {
              'path': ['Strength'],
              'value': 20.0,
            },
          },
        ],
      ),
    );
    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'New Name'},
        ],
      ),
    );

    expect(notifier.state.pendingEditCount, 3);
  });

  test(
    'two rapid saveAllPending calls issue only one write (re-entry safe)',
    () async {
      // Use a slow core so the first call is still in-flight when the second fires.
      final gate = Completer<void>();
      final core = _SlowWriteCoreService(gate.future);
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Slow Save'},
          ],
        ),
      );

      // Fire both without awaiting the first.
      final first = notifier.saveAllPending();
      final second = notifier.saveAllPending();
      gate.complete();
      await Future.wait([first, second]);

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(1));
    },
  );

  // ---------------------------------------------------------------------------
  // Other notifier methods (non-write path)
  // ---------------------------------------------------------------------------

  test('codecCompressReady follows the codec canCompress capability', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await Future<void>.delayed(Duration.zero);

    // The always-on in-process codec reports ready, so compress edits are
    // unlocked directly with no manual verification step.
    expect(notifier.state.codecStatus?.canCompress, isTrue);
    expect(notifier.state.codecCompressReady, isTrue);
  });

  test(
    'validateCodecRoundtrip sends no codec config and reports success',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      await Future<void>.delayed(Duration.zero);

      await notifier.validateCodecRoundtrip();

      final roundtrip = core.requests.lastWhere(
        (request) => request.command == 'validate_codec_roundtrip',
      );
      expect(roundtrip.payload, {'path': r'C:\tmp\saves\G1R-001.sav'});
      expect(notifier.state.lastWriteMessage, contains('roundtrip'));
    },
  );

  test('validateCodecRoundtrip surfaces failure as an error', () async {
    final core = _FailingVerifyCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await Future<void>.delayed(Duration.zero);

    await notifier.validateCodecRoundtrip();

    expect(notifier.state.error, contains('roundtrip'));
  });

  test('exhaustive typed search forwards source and node filters', () async {
    final core = _RecordingCoreService(
      typedSearchData: {
        'source': 'all',
        'offset': 0,
        'limit': 100,
        'total': 0,
        'count': 0,
        'results': <Object?>[],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    await notifier.searchTypedProperties(
      'Vector',
      limit: 100,
      includeNodes: true,
      source: 'all',
      kind: 'struct',
      type: 'StructProperty',
      editable: true,
    );

    final request = core.requests.lastWhere(
      (request) => request.command == 'search_typed_properties',
    );
    expect(request.payload, {
      'path': r'C:\tmp\saves\G1R-001.sav',
      'query': 'Vector',
      'offset': 0,
      'limit': 100,
      'includeNodes': true,
      'source': 'all',
      'kind': 'struct',
      'type': 'StructProperty',
      'editable': true,
    });
  });

  test('loadHeroAttributes searches the hero attribute subtree', () async {
    final core = _RecordingCoreService(
      typedSearchData: {
        'query': 'AttributesByGlobalId {Hero}',
        'offset': 0,
        'limit': 1000,
        'total': 2,
        'count': 2,
        'results': [
          {
            'path': [
              'm_GenericData',
              '{CharacterStates}',
              'AnyCharacterType',
              'AttributesByGlobalId',
              '{Hero}',
              'AttributeSetsByClass',
              '{/Script/G1R.AttributeSet_Health}',
              'Attributes',
              '{MaxHealth}',
              'BaseValue',
            ],
            'display': '…',
            'type': 'FloatProperty',
            'value': '64',
            'editable': true,
          },
          {
            'path': [
              'm_GenericData',
              '{CharacterStates}',
              'AnyCharacterType',
              'AttributesByGlobalId',
              '{Hero}',
              'AttributeSetsByClass',
              '{/Script/G1R.AttributeSet_Health}',
              'Attributes',
              '{MaxHealth}',
              'CurrentValue',
            ],
            'display': '…',
            'type': 'FloatProperty',
            'value': '64',
            'editable': true,
          },
        ],
      },
    );
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final result = await notifier.loadHeroAttributes();

    final search = core.requests.lastWhere(
      (request) => request.command == 'search_typed_properties',
    );
    expect(search.payload['query'], 'AttributesByGlobalId {Hero}');
    expect(search.payload['limit'], 1000);
    expect(result.error, isNull);
    expect(result.attributes, hasLength(1));
    expect(result.attributes.single.id, 'MaxHealth');
  });

  test(
    'loadHeroAttributes pages through results beyond the search cap',
    () async {
      Map<String, Object?> heroHit(String id, String leaf, String value) => {
        'path': [
          'm_GenericData',
          '{CharacterStates}',
          'AnyCharacterType',
          'AttributesByGlobalId',
          '{Hero}',
          'AttributeSetsByClass',
          '{/Script/G1R.AttributeSet_Health}',
          'Attributes',
          '{$id}',
          leaf,
        ],
        'display': '…',
        'type': 'FloatProperty',
        'value': value,
        'editable': true,
      };
      final core = _RecordingCoreService(
        typedSearchPages: [
          {
            'query': 'AttributesByGlobalId {Hero}',
            'offset': 0,
            'limit': 1000,
            'total': 2,
            'count': 1,
            'results': [heroHit('MaxHealth', 'BaseValue', '64')],
          },
          {
            'query': 'AttributesByGlobalId {Hero}',
            'offset': 1,
            'limit': 1000,
            'total': 2,
            'count': 1,
            'results': [heroHit('MaxHealth', 'CurrentValue', '64')],
          },
        ],
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      final result = await notifier.loadHeroAttributes();

      final searches = core.requests
          .where((request) => request.command == 'search_typed_properties')
          .toList();
      expect(searches, hasLength(2));
      expect(searches[0].payload['offset'], 0);
      expect(searches[1].payload['offset'], 1);
      expect(result.error, isNull);
      // Both pages were folded into one fully paired attribute.
      final attribute = result.attributes.single;
      expect(attribute.id, 'MaxHealth');
      expect(attribute.baseValue, 64);
      expect(attribute.currentValue, 64);
    },
  );

  // ---------------------------------------------------------------------------
  // Progression query methods (Task 9)
  // ---------------------------------------------------------------------------

  test('loadProgressionQuests queries the core and parses the page', () async {
    final core = _RecordingCoreService(
      progressionData: {
        'section': 'quests',
        'total': 1,
        'offset': 0,
        'limit': 100,
        'count': 1,
        'stateCounts': {'Running': 1},
        'quests': [
          {
            'questClass': '/Script/Angelscript.Quest_X',
            'id': 'Quest_X',
            'group': 'X',
            'name': '',
            'currentState': 'EQuestState::Running',
            'statePath': [
              'QuestDataByClass',
              '{/Script/Angelscript.Quest_X}',
              'CurrentState',
            ],
            'writable': true,
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadProgressionQuests(query: 'x');

    expect(page.error, isNull);
    expect(page.quests.single.id, 'Quest_X');
    final call = core.requests.singleWhere(
      (r) => r.command == 'query_progression',
    );
    expect(call.payload['section'], 'quests');
    expect(call.payload['query'], 'x');
    expect(call.payload['path'], r'C:\tmp\saves\G1R-001.sav');
  });

  test(
    'loadProgressionQuests passes state and group params to the core',
    () async {
      final core = _RecordingCoreService(
        progressionData: {
          'section': 'quests',
          'total': 0,
          'offset': 0,
          'limit': 50,
          'count': 0,
          'stateCounts': <String, Object?>{},
          'groupCounts': <String, Object?>{},
          'quests': <Object?>[],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      await notifier.loadProgressionQuests(
        state: 'Running',
        group: 'OldCamp',
        limit: 50,
      );

      final call = core.requests.lastWhere(
        (r) => r.command == 'query_progression',
      );
      expect(call.payload['state'], 'Running');
      expect(call.payload['group'], 'OldCamp');

      // Null/empty filters must NOT appear in the payload.
      await notifier.loadProgressionQuests(limit: 50);
      final callNoFilter = core.requests.lastWhere(
        (r) => r.command == 'query_progression',
      );
      expect(callNoFilter.payload.containsKey('state'), isFalse);
      expect(callNoFilter.payload.containsKey('group'), isFalse);
    },
  );

  test(
    'loadProgressionTutorials queries the dedicated tutorials section',
    () async {
      final core = _RecordingCoreService(
        progressionData: {
          'section': 'tutorials',
          'total': 1,
          'offset': 0,
          'limit': 100,
          'count': 1,
          'quests': [
            {
              'questClass':
                  '/Script/Angelscript.Quest_Tutorials_Tut_CombatBasics',
              'id': 'Quest_Tutorials_Tut_CombatBasics',
              'group': 'Tutorials',
              'name': 'Tut_CombatBasics',
              'currentState': 'EQuestState::Running',
              'statePath': [
                'QuestDataByClass',
                '{/Script/Angelscript.Quest_Tutorials_Tut_CombatBasics}',
                'CurrentState',
              ],
              'writable': true,
            },
          ],
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      final page = await notifier.loadProgressionTutorials();

      expect(page.error, isNull);
      expect(page.quests.single.name, 'Tut_CombatBasics');
      final call = core.requests.lastWhere(
        (request) => request.command == 'query_progression',
      );
      expect(call.payload['section'], 'tutorials');
      expect(call.payload['offset'], 0);
      expect(call.payload['limit'], 100);
      expect(call.payload.containsKey('query'), isFalse);
      expect(call.payload.containsKey('group'), isFalse);
    },
  );

  test('loadStoryState honors a pinned save path', () async {
    final core = _RecordingCoreService(
      progressionData: {
        'section': 'story',
        'total': 0,
        'offset': 0,
        'limit': 1000,
        'count': 0,
        'catalogTotal': 470,
        'storedTotal': 0,
        'unsetTotal': 470,
        'unknownStoredTotal': 0,
        'entries': <Object?>[],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-002.sav');

    final page = await notifier.loadStoryState(
      includeUnset: true,
      path: r'C:\tmp\saves\G1R-001.sav',
    );

    expect(page.error, isNull);
    final call = core.requests.lastWhere(
      (request) => request.command == 'query_progression',
    );
    expect(call.payload['section'], 'story');
    expect(call.payload['includeUnset'], isTrue);
    expect(call.payload['path'], r'C:\tmp\saves\G1R-001.sav');
  });

  test('progression loaders surface core errors inline', () async {
    // The default _RecordingCoreService returns ok:false for query_progression
    // (no progressionData set), so the loader should surface the error inline.
    // All progression loaders share _queryProgression; loadKnowledgeEntries is
    // the exercised representative.
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadKnowledgeEntries('OC_STT_Diego');

    expect(page.error, isNotNull);
  });

  test('memory-event edit stays pending until global Save', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    final writesBefore = core.requests
        .where((request) => request.command == 'write_save')
        .length;
    final edit = MemoryEventEdit.remove(
      arrayPath: const [
        'LongTermMemoryByGlobalId',
        '{Hero}',
        'MemorizedEvents',
      ],
      index: 4,
    );

    notifier.setPendingMemoryEventEdit('Hero', edit);

    expect(
      core.requests.where((request) => request.command == 'write_save'),
      hasLength(writesBefore),
    );
    expect(notifier.state.pendingEdits, hasLength(1));
    expect(notifier.pendingMemoryEventEdit('Hero')?.index, 4);

    expect(await notifier.saveAllPending(), isTrue);
    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect((write.payload['edits'] as List).single, edit.toEditJson());
  });

  test(
    'memory-event removals stack uniquely and save index-descending as singleton writes',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];

      expect(
        notifier.setPendingMemoryEventEdit(
          'Hero',
          const MemoryEventEdit.remove(arrayPath: path, index: 4),
        ),
        isTrue,
      );
      expect(
        notifier.setPendingMemoryEventEdit(
          'Hero',
          const MemoryEventEdit.remove(arrayPath: path, index: 9),
        ),
        isTrue,
      );
      expect(
        notifier.setPendingMemoryEventEdit(
          'Hero',
          const MemoryEventEdit.remove(arrayPath: path, index: 6),
        ),
        isTrue,
      );
      // Re-queuing the same removal is idempotent, not a second splice.
      expect(
        notifier.setPendingMemoryEventEdit(
          'Hero',
          const MemoryEventEdit.remove(arrayPath: path, index: 6),
        ),
        isTrue,
      );

      expect(
        notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index),
        [9, 6, 4],
      );
      expect(notifier.pendingEditCount, 3);

      expect(await notifier.saveAllPending(), isTrue);
      final writes = core.requests
          .where((request) => request.command == 'write_save')
          .toList();
      expect(writes, hasLength(3));
      expect(
        writes.map(
          (write) =>
              ((((write.payload['edits'] as List).single as Map)['value']
                          as Map)['index']
                      as num)
                  .toInt(),
        ),
        [9, 6, 4],
      );
      expect(writes.map((write) => write.payload['backup']), [
        true,
        false,
        false,
      ]);
      expect(
        writes.every((write) => (write.payload['edits'] as List).length == 1),
        isTrue,
      );
    },
  );

  test(
    'one pending memory-event removal can be undone without clearing peers',
    () async {
      final notifier = EditorNotifier(
        _RecordingCoreService(),
        saveDir: r'C:\tmp\saves',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];
      for (final index in [2, 8, 5]) {
        notifier.setPendingMemoryEventEdit(
          'Hero',
          MemoryEventEdit.remove(arrayPath: path, index: index),
        );
      }

      notifier.clearPendingMemoryEventEdit('Hero', index: 5);

      expect(
        notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index),
        [8, 2],
      );
      expect(notifier.pendingEditCount, 2);
      notifier.clearPendingMemoryEventEdit('Hero');
      expect(notifier.pendingMemoryEventEdits('Hero'), isEmpty);
    },
  );

  test('memory-event duplicate is exclusive with pending removals', () async {
    final notifier = EditorNotifier(
      _RecordingCoreService(),
      saveDir: r'C:\tmp\saves',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];
    expect(
      notifier.setPendingMemoryEventEdit(
        'Hero',
        const MemoryEventEdit.remove(arrayPath: path, index: 3),
      ),
      isTrue,
    );

    expect(
      notifier.setPendingMemoryEventEdit(
        'Hero',
        const MemoryEventEdit.duplicate(arrayPath: path, index: 7),
      ),
      isFalse,
    );
    expect(notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index), [
      3,
    ]);
  });

  test(
    'saveAllPending rejects remove plus duplicate for the same array',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];
      notifier.setPendingEdit(
        'raw-memory-structural-edits',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.arrayRemove',
              'value': {'path': path, 'index': 2},
            },
            {
              'path': 'private.typed.arrayDuplicate',
              'value': {'path': path, 'index': 7},
            },
          ],
        ),
      );

      expect(await notifier.saveAllPending(), isFalse);
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      expect(notifier.state.error, contains('same array'));
    },
  );

  test(
    'saveAllPending rejects a typed descendant of a removed event array',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];
      notifier.setPendingMemoryEventEdit(
        'Hero',
        const MemoryEventEdit.remove(arrayPath: path, index: 7),
      );
      notifier.setPendingEdit(
        'all-data-memory-time',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': [...path, '[2]', 'Time', 'TotalSeconds'],
                'value': 12.0,
              },
            },
          ],
        ),
      );

      expect(await notifier.saveAllPending(), isFalse);
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      expect(notifier.state.error, contains('structural event change'));
    },
  );

  test(
    'saveAllPending rejects memory-event removal alongside NPC revive',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      const path = ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'];
      notifier.setPendingMemoryEventEdit(
        'Hero',
        const MemoryEventEdit.remove(arrayPath: path, index: 7),
      );
      notifier.setPendingNpcRevive('Asghan-1');

      expect(await notifier.saveAllPending(), isFalse);
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      expect(notifier.state.error, contains('same array'));
    },
  );

  test('clearing a pending memory event performs no write', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    final writesBefore = core.requests
        .where((request) => request.command == 'write_save')
        .length;

    notifier.setPendingMemoryEventEdit(
      'Hero',
      MemoryEventEdit.duplicate(arrayPath: const ['MemorizedEvents'], index: 0),
    );
    notifier.clearPendingMemoryEventEdit('Hero');

    expect(notifier.pendingMemoryEventEdit('Hero'), isNull);
    expect(notifier.state.pendingEdits, isEmpty);
    expect(
      core.requests.where((request) => request.command == 'write_save'),
      hasLength(writesBefore),
    );
  });

  test('first knowledge entry stays pending until global Save', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    final writesBefore = core.requests
        .where((request) => request.command == 'write_save')
        .length;
    final edit = KnowledgeEntryEdit.add(
      character: 'NewNpc',
      entry: 'Info_Whatslife',
    ).toEditJson();

    notifier.setPendingEdit(
      'progression.knowledge',
      PendingSaveEdit(edits: [edit]),
    );

    expect(
      core.requests.where((request) => request.command == 'write_save'),
      hasLength(writesBefore),
    );
    expect(await notifier.saveAllPending(), isTrue);
    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect((write.payload['edits'] as List).single, edit);
  });

  // ---------------------------------------------------------------------------
  // Profile switcher (selectProfile)
  // ---------------------------------------------------------------------------

  test(
    'selectProfile filters visibleSaves and moves selection to that profile',
    () async {
      // Two profiles: profile 0 has G1R-001, profile 1 has G1R-002.
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Save A',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Save B',
              'persistentProfileId': 1,
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
            {
              'profileId': 1,
              'profileName': '1',
              'quickSaveSlots': <String>[],
              'autoSaveSlots': <String>[],
              'savedSlots': ['G1R-002'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      // Initial selection should be profile 0's save (first after sort).
      // Both profiles exist so visibleSaves should only show profile 0 saves.
      expect(notifier.state.profiles.length, 2);
      expect(
        notifier.state.visibleSaves.map((s) => s.slot),
        contains('G1R-001'),
      );
      expect(
        notifier.state.visibleSaves.map((s) => s.slot),
        isNot(contains('G1R-002')),
      );

      // Switch to profile 1.
      await notifier.selectProfile(1);

      // visibleSaves should now only show profile 1's save.
      expect(
        notifier.state.visibleSaves.map((s) => s.slot),
        contains('G1R-002'),
      );
      expect(
        notifier.state.visibleSaves.map((s) => s.slot),
        isNot(contains('G1R-001')),
      );
      // Selection moved to profile 1's save.
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
    },
  );

  test(
    'selectProfile selects a real save after a missing-only profile',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'MISSING',
              'fileSize': 0,
              'sha1': '',
              'status': 'missing',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Save B',
              'persistentProfileId': 1,
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
            {
              'profileId': 1,
              'profileName': '1',
              'quickSaveSlots': <String>[],
              'autoSaveSlots': <String>[],
              'savedSlots': ['G1R-002'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      expect(notifier.state.selectedPath, isNull);

      await notifier.selectProfile(1);

      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
      expect(notifier.state.inspection, isNotNull);
    },
  );

  test(
    'selectProfile with pending edits is blocked and sets an error',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Save A',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Save B',
              'persistentProfileId': 1,
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
            {
              'profileId': 1,
              'profileName': '1',
              'quickSaveSlots': <String>[],
              'autoSaveSlots': <String>[],
              'savedSlots': ['G1R-002'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Draft'},
          ],
        ),
      );

      final profileBefore = notifier.state.selectedProfileId;
      await notifier.selectProfile(1);

      // Profile must not have changed.
      expect(notifier.state.selectedProfileId, profileBefore);
      // An error must be set.
      expect(notifier.state.error, isNotNull);
      expect(notifier.state.error, contains('unsaved changes'));
    },
  );

  test(
    'refresh keeps selectedProfileId when the profile still exists',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Save A',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Save B',
              'persistentProfileId': 1,
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
            {
              'profileId': 1,
              'profileName': '1',
              'quickSaveSlots': <String>[],
              'autoSaveSlots': <String>[],
              'savedSlots': ['G1R-002'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      // Select profile 1 explicitly.
      await notifier.selectProfile(1);
      expect(notifier.state.selectedProfileId, 1);

      // Refresh — profile 1 still exists in scan data.
      await notifier.refresh();

      // selectedProfileId must be preserved.
      expect(notifier.state.selectedProfileId, 1);
    },
  );

  test(
    'external saves accumulate, persist, and survive refresh without a profile',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Folder save',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-001'],
              'difficultyPreset': 'DifficultyPreset_Standard',
            },
          ],
          'activeProfileId': 0,
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
        fileExists: (_) => true,
      );
      await pumpEventQueue();

      const externalPath = r'D:\archive\My-Gothic-Save.sav';
      await notifier.loadExternalSave(externalPath);

      expect(notifier.state.selectedPath, externalPath);
      expect(notifier.state.selectedSave?.isExternal, isTrue);
      expect(notifier.state.selectedSave?.persistentProfileId, isNull);
      expect(notifier.state.otherSavesSelected, isTrue);
      expect(notifier.state.otherSaves.map((save) => save.path), [
        externalPath,
      ]);
      expect(notifier.state.visibleSaves.map((save) => save.path), [
        externalPath,
      ]);
      expect(store.settings.externalSavePaths, [externalPath]);
      expect(
        notifier.state.activeProfile,
        isNull,
        reason: 'detached saves must not borrow the folder profile difficulty',
      );

      // Reopening that same detached file through a differently-cased,
      // normalizable Windows path must retain the SaveSlot's canonical path.
      // Otherwise exact state accessors can no longer find the selected offer.
      await notifier.loadExternalSave(r'd:/ARCHIVE/./my-gothic-save.SAV');
      expect(notifier.state.selectedPath, externalPath);
      expect(notifier.state.selectedSave?.isExternal, isTrue);
      expect(
        notifier.state.saves.where((save) => save.isExternal),
        hasLength(1),
      );

      const secondExternalPath = r'E:\archive\Second.sav';
      await notifier.loadExternalSave(secondExternalPath);
      expect(
        notifier.state.otherSaves.map((save) => save.path),
        containsAll([externalPath, secondExternalPath]),
      );
      expect(store.settings.externalSavePaths, [
        externalPath,
        secondExternalPath,
      ]);

      await notifier.refresh();
      expect(notifier.state.selectedPath, secondExternalPath);
      expect(notifier.state.selectedSave?.isExternal, isTrue);
      expect(
        notifier.state.saves.where((save) => save.isExternal),
        hasLength(2),
      );
    },
  );

  test(
    'refresh restores persisted external saves and prunes missing files',
    () async {
      const existingPath = r'D:\archive\Existing.sav';
      const missingPath = r'D:\archive\Missing.sav';
      final store = _MemoryEditorSettingsStore(
        const EditorSettings(externalSavePaths: [existingPath, missingPath]),
      );
      final notifier = EditorNotifier(
        _RecordingCoreService(),
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
        fileExists: (path) => _sameTestPath(path, existingPath),
      );
      await pumpEventQueue();

      expect(notifier.state.externalSavePaths, [existingPath]);
      expect(store.settings.externalSavePaths, [existingPath]);
      expect(
        notifier.state.saves.where((save) => save.isExternal).single.path,
        existingPath,
      );

      await notifier.selectOtherSaves();
      expect(notifier.state.selectedPath, existingPath);
      expect(notifier.state.visibleSaves.single.path, existingPath);

      expect(await notifier.removeOtherSave(existingPath), isTrue);
      expect(notifier.state.otherSaves, isEmpty);
      expect(store.settings.externalSavePaths, isEmpty);
      expect(store.settings.hiddenOtherSavePaths, [existingPath]);
    },
  );

  test(
    'failed folder scan still restores external saves and prunes missing paths',
    () async {
      const existingPath = r'D:\archive\Existing.sav';
      const missingPath = r'D:\archive\Missing.sav';
      final store = _MemoryEditorSettingsStore(
        const EditorSettings(externalSavePaths: [existingPath, missingPath]),
      );
      final notifier = EditorNotifier(
        _FailingScanCoreService(),
        saveDir: r'C:\missing\saves',
        settingsStore: store,
        fileExists: (path) => _sameTestPath(path, existingPath),
      );
      await pumpEventQueue();

      expect(notifier.state.externalSavePaths, [existingPath]);
      expect(store.settings.externalSavePaths, [existingPath]);
      expect(notifier.state.otherSavesSelected, isTrue);
      expect(notifier.state.otherSaves.single.path, existingPath);
      expect(notifier.state.selectedSave?.path, existingPath);
      expect(notifier.state.error, isNotNull);
    },
  );

  test(
    'thrown folder scan still restores external saves and prunes missing paths',
    () async {
      const existingPath = r'D:\archive\Existing.sav';
      const missingPath = r'D:\archive\Missing.sav';
      final store = _MemoryEditorSettingsStore(
        const EditorSettings(externalSavePaths: [existingPath, missingPath]),
      );
      final notifier = EditorNotifier(
        _ThrowingScanCoreService(),
        saveDir: r'C:\missing\saves',
        settingsStore: store,
        fileExists: (path) => _sameTestPath(path, existingPath),
      );
      await pumpEventQueue();

      expect(notifier.state.externalSavePaths, [existingPath]);
      expect(store.settings.externalSavePaths, [existingPath]);
      expect(notifier.state.otherSaves.single.path, existingPath);
      expect(notifier.state.selectedSave?.path, existingPath);
      expect(notifier.state.error, contains('native scan failed'));
    },
  );

  test('refresh canonicalizes a retained Windows save selection', () async {
    const openedPath = r'c:\archive\loose.sav';
    const scannedPath = r'C:\ARCHIVE\Loose.sav';
    final core = _RecordingCoreService();
    final store = _MemoryEditorSettingsStore();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\tmp\saves',
      settingsStore: store,
      fileExists: (_) => true,
    );
    await pumpEventQueue();

    await notifier.loadExternalSave(openedPath);
    expect(notifier.state.selectedPath, openedPath);

    core.scanData
      ..['saves'] = [
        {
          'path': scannedPath,
          'slot': 'Loose',
          'format': 'GSAV',
          'fileSize': 100,
          'sha1': 'canonical',
          'status': 'ok',
        },
      ]
      ..['profiles'] = <Object?>[];
    await notifier.refresh();

    expect(notifier.state.selectedPath, scannedPath);
    expect(notifier.state.selectedSave?.path, scannedPath);
    expect(notifier.state.selectedSave?.isExternal, isFalse);
    expect(notifier.state.externalSavePaths, isEmpty);
    expect(store.settings.externalSavePaths, isEmpty);
  });

  test(
    'removed external save stays hidden when a later scan discovers it',
    () async {
      const path = r'D:\archive\Loose.sav';
      final core = _RecordingCoreService();
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
        fileExists: (_) => true,
      );
      await pumpEventQueue();

      await notifier.loadExternalSave(path);
      expect(await notifier.removeOtherSave(path), isTrue);
      expect(store.settings.externalSavePaths, isEmpty);
      expect(store.settings.hiddenOtherSavePaths, [path]);

      core.scanData['saves'] = [
        {
          'path': path,
          'slot': 'Loose',
          'format': 'GSAV',
          'fileSize': 100,
          'sha1': 'now-scanned',
          'status': 'ok',
        },
      ];
      await notifier.refresh();

      expect(notifier.state.otherSaves, isEmpty);
      expect(store.settings.hiddenOtherSavePaths, [path]);

      // Explicitly opening it again is the user's opt-in to re-add it.
      await notifier.loadExternalSave(path);
      expect(notifier.state.otherSaves.single.path, path);
      expect(store.settings.hiddenOtherSavePaths, isEmpty);
    },
  );

  test(
    'external save with a local slot basename remains profileless',
    () async {
      const externalPath = r'D:\archive\G1R-001.sav';
      final store = _MemoryEditorSettingsStore(
        const EditorSettings(externalSavePaths: [externalPath]),
      );
      final notifier = EditorNotifier(
        _RecordingCoreService(
          scanData: {
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 100,
                'sha1': 'local',
                'status': 'ok',
                'persistentProfileId': 0,
              },
            ],
            'profiles': [
              {
                'profileId': 0,
                'profileName': '0',
                'savedSlots': ['G1R-001'],
              },
            ],
            'activeProfileId': 0,
          },
        ),
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
        fileExists: (path) => _sameTestPath(path, externalPath),
      );
      await pumpEventQueue();

      final external = notifier.state.saves.singleWhere(
        (save) => save.isExternal,
      );
      expect(notifier.state.profileIdForSave(external), isNull);
      expect(notifier.state.otherSaves, contains(external));
    },
  );

  test(
    'removing a scanned Other save persists a tombstone across refresh',
    () async {
      const path = r'C:\tmp\saves\G1R-007.sav';
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': path,
              'slot': 'G1R-007',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'other',
              'status': 'ok',
              'playerSaveName': 'Loose save',
            },
          ],
          'profiles': <Object?>[],
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );
      await pumpEventQueue();
      await notifier.selectOtherSaves();
      expect(notifier.state.visibleSaves.single.path, path);

      expect(await notifier.removeOtherSave(path), isTrue);
      expect(notifier.state.otherSaves, isEmpty);
      expect(store.settings.hiddenOtherSavePaths, [path]);

      await notifier.refresh();
      expect(notifier.state.otherSaves, isEmpty);
      expect(store.settings.hiddenOtherSavePaths, [path]);

      final restarted = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );
      await pumpEventQueue();
      expect(restarted.state.otherSaves, isEmpty);

      // Choosing the still-existing file through Open file explicitly re-adds
      // it and clears the persisted tombstone.
      await restarted.loadExternalSave(path);
      expect(restarted.state.otherSaves.single.path, path);
      expect(store.settings.hiddenOtherSavePaths, isEmpty);
    },
  );

  test(
    'opening a scanned path normalizes Windows casing and selects its profile',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'persistentProfileId': 1,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-001'],
            },
            {
              'profileId': 1,
              'profileName': '1',
              'savedSlots': ['G1R-002'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      await notifier.loadExternalSave(r'c:/TMP/saves/./g1r-002.SAV');

      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
      expect(notifier.state.selectedProfileId, 1);
      expect(notifier.state.activeProfile?.profileId, 1);
      expect(
        notifier.state.visibleSaves.map((save) => save.slot),
        orderedEquals(['G1R-002']),
      );
      expect(notifier.state.saves.where((save) => save.isExternal), isEmpty);
    },
  );

  test(
    'failed external open restores the previous save and inspection',
    () async {
      final core = _RejectExternalInspectCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-001'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );
      await pumpEventQueue();
      final previousInspection = notifier.state.inspection;
      final previousBackups = notifier.state.backups;

      await notifier.loadExternalSave(r'D:\archive\invalid.sav');

      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-001.sav');
      expect(notifier.state.inspection, same(previousInspection));
      expect(notifier.state.backups, same(previousBackups));
      expect(notifier.state.activeProfile?.profileId, 0);
      expect(notifier.state.otherSaves, isEmpty);
      expect(notifier.state.otherSavesSelected, isFalse);
      expect(notifier.state.saves.where((save) => save.isExternal), isEmpty);
      expect(store.settings.externalSavePaths, isEmpty);
      expect(notifier.state.error, contains('invalid external file'));
    },
  );

  test(
    'assignSelectedSaveToProfile calls atomic profile command and rescans',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-006.sav',
              'slot': 'G1R-006',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Move me',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-006'],
            },
            {'profileId': 1, 'profileName': '1', 'savedSlots': <String>[]},
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      final ok = await notifier.assignSelectedSaveToProfile(1);

      expect(ok, isTrue);
      final request = core.requests.lastWhere(
        (request) => request.command == 'assign_save_profile',
      );
      expect(request.payload['path'], r'C:\tmp\saves\G1R-006.sav');
      expect(
        request.payload['persistentPath'],
        r'C:\tmp\saves\PersistentDataList.sav',
      );
      expect(request.payload['profileId'], 1);
      expect(request.payload['backup'], isTrue);
      expect(notifier.state.selectedProfileId, 1);
      expect(notifier.state.selectedSave?.persistentProfileId, 1);
    },
  );

  test(
    'missing profile reference stays visible but is never inspected or selected',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-009.sav',
              'slot': 'G1R-009',
              'format': 'MISSING',
              'fileSize': 0,
              'sha1': '',
              'status': 'missing',
              'persistentPlayerSaveName': 'Lost save',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-009'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      expect(notifier.state.visibleSaves.single.isMissing, isTrue);
      expect(notifier.state.selectedPath, isNull);
      expect(notifier.state.inspection, isNull);
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );

      await notifier.inspect(r'C:\tmp\saves\G1R-009.sav');
      expect(notifier.state.selectedPath, isNull);
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );
    },
  );

  test(
    'removeSaveFromProfile cleans a missing reference and sends no inspect',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-009.sav',
              'slot': 'G1R-009',
              'format': 'MISSING',
              'fileSize': 0,
              'sha1': '',
              'status': 'missing',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'quickSaveSlots': ['G1R-009'],
              'autoSaveSlots': ['G1R-009'],
              'savedSlots': ['G1R-009'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      final ok = await notifier.removeSaveFromProfile(
        slot: 'G1R-009',
        profileId: 0,
      );

      expect(ok, isTrue);
      final request = core.requests.lastWhere(
        (request) => request.command == 'remove_save_from_profile',
      );
      expect(
        request.payload['persistentPath'],
        r'C:\tmp\saves\PersistentDataList.sav',
      );
      expect(request.payload['slot'], 'G1R-009');
      expect(request.payload['profileId'], 0);
      expect(request.payload['backup'], isTrue);
      expect(notifier.state.saves, isEmpty);
      expect(notifier.state.profiles.single.savedSlots, isEmpty);
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );
    },
  );

  test('removeSaveFromProfile keeps an existing save as unassigned', () async {
    final core = _RecordingCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-006.sav',
            'slot': 'G1R-006',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'a',
            'status': 'ok',
            'playerSaveName': 'Keep me',
            'persistentProfileId': 0,
          },
        ],
        'profiles': [
          {
            'profileId': 0,
            'profileName': '0',
            'savedSlots': ['G1R-006'],
          },
        ],
        'activeProfileId': 0,
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();

    final ok = await notifier.removeSaveFromProfile(
      slot: 'G1R-006',
      profileId: 0,
    );

    expect(ok, isTrue);
    expect(notifier.state.saves, hasLength(1));
    expect(notifier.state.selectedSave, isNull);
    expect(notifier.state.otherSaves.map((save) => save.slot), ['G1R-006']);
    expect(notifier.state.visibleSaves, isEmpty);
    expect(notifier.state.profiles.single.savedSlots, isEmpty);
  });

  test(
    'deleteSave removes the registered file and refreshes selection',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-006.sav',
              'slot': 'G1R-006',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Delete me',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-006'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      final ok = await notifier.deleteSave(slot: 'G1R-006', profileId: 0);

      expect(ok, isTrue);
      final request = core.requests.lastWhere(
        (request) => request.command == 'delete_save',
      );
      expect(request.payload['path'], r'C:\tmp\saves\G1R-006.sav');
      expect(
        request.payload['persistentPath'],
        r'C:\tmp\saves\PersistentDataList.sav',
      );
      expect(request.payload['slot'], 'G1R-006');
      expect(request.payload['profileId'], 0);
      expect(request.payload['backup'], isTrue);
      expect(notifier.state.saves, isEmpty);
      expect(notifier.state.profiles.single.savedSlots, isEmpty);
      expect(notifier.state.selectedPath, isNull);
      expect(notifier.state.inspection, isNull);
      expect(
        notifier.state.deletedSaveRecovery?.targetPath,
        r'C:\tmp\saves\G1R-006.sav',
      );
      expect(
        notifier.state.deletedSaveRecovery?.backupPath,
        r'C:\tmp\saves\goresave_backups\G1R-006.sav.bak.2',
      );
      expect(
        notifier.state.deletedSaveRecovery?.persistentPostDeleteSha1,
        'post-delete-profile-sha',
      );
      expect(
        notifier.state.deletedSaveRecovery?.deletedSaveSha1,
        'deleted-save-sha',
      );
      expect(
        notifier.state.deletedSaveRecovery?.deletedPersistentSha1,
        'deleted-persistent-sha',
      );

      await notifier.restoreDeletedSave();

      final restore = core.requests.lastWhere(
        (request) => request.command == 'restore_deleted_save',
      );
      expect(restore.payload['path'], r'C:\tmp\saves\G1R-006.sav');
      expect(
        restore.payload['backupPath'],
        r'C:\tmp\saves\goresave_backups\G1R-006.sav.bak.2',
      );
      expect(
        restore.payload['expectedPersistentSha1'],
        'post-delete-profile-sha',
      );
      expect(restore.payload['expectedSaveSha1'], 'deleted-save-sha');
      expect(
        restore.payload['expectedPersistentBackupSha1'],
        'deleted-persistent-sha',
      );
      expect(notifier.state.deletedSaveRecovery, isNull);
    },
  );

  test('restoreDeletedSave preserves pending edits', () async {
    final core = _RecordingCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-006.sav',
            'slot': 'G1R-006',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'a',
            'status': 'ok',
            'persistentProfileId': 0,
          },
        ],
        'profiles': [
          {
            'profileId': 0,
            'profileName': '0',
            'savedSlots': ['G1R-006'],
          },
        ],
        'activeProfileId': 0,
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();
    expect(await notifier.deleteSave(slot: 'G1R-006', profileId: 0), isTrue);
    notifier.setPendingEdit(
      'draft',
      const PendingSaveEdit(
        edits: [
          {'op': 'public.setName', 'value': 'Keep me'},
        ],
      ),
    );

    await notifier.restoreDeletedSave();

    expect(notifier.state.pendingEdits, contains('draft'));
    expect(notifier.state.deletedSaveRecovery, isNotNull);
    expect(
      core.requests.where(
        (request) => request.command == 'restore_deleted_save',
      ),
      isEmpty,
    );
  });

  test('deleteSave preserves an existing one-click recovery', () async {
    final core = _RecordingCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-006.sav',
            'slot': 'G1R-006',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'a',
            'status': 'ok',
            'persistentProfileId': 0,
          },
          {
            'path': r'C:\tmp\saves\G1R-007.sav',
            'slot': 'G1R-007',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'b',
            'status': 'ok',
            'persistentProfileId': 0,
          },
        ],
        'profiles': [
          {
            'profileId': 0,
            'profileName': '0',
            'savedSlots': ['G1R-006', 'G1R-007'],
          },
        ],
        'activeProfileId': 0,
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();

    expect(await notifier.deleteSave(slot: 'G1R-006', profileId: 0), isTrue);
    final recovery = notifier.state.deletedSaveRecovery;

    expect(await notifier.deleteSave(slot: 'G1R-007', profileId: 0), isFalse);
    expect(notifier.state.deletedSaveRecovery, same(recovery));
    expect(
      core.requests.where((request) => request.command == 'delete_save'),
      hasLength(1),
    );
  });

  test(
    'selecting another save keeps deleted-save recovery reachable',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-006.sav',
              'slot': 'G1R-006',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'Delete me',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-007.sav',
              'slot': 'G1R-007',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Keep me',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-006', 'G1R-007'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      expect(await notifier.deleteSave(slot: 'G1R-006', profileId: 0), isTrue);
      expect(notifier.state.deletedSaveRecovery, isNotNull);

      await notifier.inspect(r'C:\tmp\saves\G1R-007.sav');

      expect(notifier.state.lastWriteMessage, isNull);
      expect(notifier.state.deletedSaveRecovery, isNotNull);
      notifier.dismissDeletedSaveRecovery();
      expect(notifier.state.deletedSaveRecovery, isNull);
    },
  );

  test(
    'all unassigned saves stay out of profiles and collect in Other saves',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'a',
              'status': 'ok',
              'playerSaveName': 'First detached',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'b',
              'status': 'ok',
              'playerSaveName': 'Second detached',
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-003.sav',
              'slot': 'G1R-003',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'c',
              'status': 'ok',
              'playerSaveName': 'Other profile',
              'persistentProfileId': 1,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-001', 'G1R-002'],
            },
            {
              'profileId': 1,
              'profileName': '1',
              'savedSlots': ['G1R-003'],
            },
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();

      expect(
        await notifier.removeSaveFromProfile(slot: 'G1R-001', profileId: 0),
        isTrue,
      );
      expect(notifier.state.otherSaves.map((save) => save.slot), ['G1R-001']);
      expect(
        notifier.state.visibleSaves.map((save) => save.slot),
        orderedEquals(['G1R-002']),
      );

      await notifier.selectProfile(1);
      expect(
        notifier.state.visibleSaves.map((save) => save.slot),
        orderedEquals(['G1R-003']),
      );
      expect(
        notifier.state.visibleSaves.any(
          (save) => save.persistentProfileId == null,
        ),
        isFalse,
      );

      await notifier.selectProfile(0);
      expect(
        await notifier.removeSaveFromProfile(slot: 'G1R-002', profileId: 0),
        isTrue,
      );
      expect(
        notifier.state.otherSaves.map((save) => save.slot),
        containsAll(['G1R-001', 'G1R-002']),
      );
      expect(
        notifier.state.saves
            .where((save) => save.persistentProfileId == null)
            .map((save) => save.slot),
        containsAll(['G1R-001', 'G1R-002']),
      );

      await notifier.selectOtherSaves();
      expect(notifier.state.otherSavesSelected, isTrue);
      expect(
        notifier.state.visibleSaves.map((save) => save.slot),
        containsAll(['G1R-001', 'G1R-002']),
      );
      expect(notifier.state.selectedSave?.slot, anyOf('G1R-001', 'G1R-002'));
      expect(notifier.state.activeProfile, isNull);
    },
  );

  test(
    'assignSelectedSaveToProfile imports detached file into first free slot',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'folder',
              'status': 'ok',
              'persistentProfileId': 0,
            },
          ],
          'profiles': [
            {'profileId': 0, 'profileName': '0', 'savedSlots': <String>[]},
          ],
          'activeProfileId': 0,
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );
      await pumpEventQueue();
      await notifier.loadExternalSave(r'D:\archive\Detached.sav');

      final ok = await notifier.assignSelectedSaveToProfile(0);

      expect(ok, isTrue);
      final request = core.requests.lastWhere(
        (request) => request.command == 'assign_save_profile',
      );
      expect(request.payload['path'], r'D:\archive\Detached.sav');
      expect(request.payload['destinationPath'], r'C:\tmp\saves\G1R-002.sav');
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
      expect(notifier.state.selectedSave?.isExternal, isFalse);
      expect(notifier.state.selectedSave?.persistentProfileId, 0);
      expect(notifier.state.otherSavesSelected, isFalse);
      expect(store.settings.externalSavePaths, isEmpty);
      expect(
        notifier.state.saves.any(
          (save) => save.path == r'D:\archive\Detached.sav',
        ),
        isFalse,
      );
    },
  );

  test(
    'assignSelectedSaveToProfile preserves a free conventional source slot',
    () async {
      final core = _RecordingCoreService(
        scanData: {
          'saves': <Object?>[],
          'profiles': [
            {'profileId': 0, 'profileName': '0', 'savedSlots': <String>[]},
          ],
          'activeProfileId': 0,
        },
      );
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await pumpEventQueue();
      await notifier.loadExternalSave(r'D:\archive\G1R-042.sav');

      final ok = await notifier.assignSelectedSaveToProfile(0);

      expect(ok, isTrue);
      final request = core.requests.lastWhere(
        (request) => request.command == 'assign_save_profile',
      );
      expect(request.payload['destinationPath'], r'C:\tmp\saves\G1R-042.sav');
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-042.sav');
    },
  );

  test(
    'failed detached import restores external entry and selection',
    () async {
      final core = _FailingAssignProfileCoreService(
        scanData: {
          'saves': <Object?>[],
          'profiles': [
            {'profileId': 0, 'profileName': '0', 'savedSlots': <String>[]},
          ],
          'activeProfileId': 0,
        },
      );
      final store = _MemoryEditorSettingsStore();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        settingsStore: store,
      );
      await pumpEventQueue();
      const externalPath = r'D:\archive\Detached.sav';
      await notifier.loadExternalSave(externalPath);

      final ok = await notifier.assignSelectedSaveToProfile(0);

      expect(ok, isFalse);
      expect(notifier.state.error, contains('import failed'));
      expect(notifier.state.selectedPath, externalPath);
      expect(notifier.state.selectedSave?.isExternal, isTrue);
      expect(notifier.state.otherSavesSelected, isTrue);
      expect(store.settings.externalSavePaths, [externalPath]);
      expect(
        notifier.state.saves.where((save) => save.isExternal),
        hasLength(1),
      );
    },
  );

  test('loadNpcAttributes sends id+path and parses typed rows', () async {
    final core = _NpcAttributesCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'a',
            'status': 'ok',
            'playerSaveName': 'Save A',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();

    final result = await notifier.loadNpcAttributes('Lizard-1');

    expect(result.error, isNull);
    expect(result.attributes, hasLength(1));
    final row = result.attributes.single;
    expect(row.key, 'Health');
    expect(row.base, 25.6);
    expect(row.current, 25.6);
    expect(row.basePath.last, 'BaseValue');
    expect(row.currentPath.last, 'CurrentValue');

    final request = core.requests.lastWhere(
      (r) => r.command == 'private.npc.attributes',
    );
    expect(request.payload['id'], 'Lizard-1');
    expect(request.payload['path'], r'C:\tmp\saves\G1R-001.sav');
  });

  test('loadNpcAttributes surfaces a core error inline', () async {
    final core = _RecordingCoreService(
      scanData: {
        'saves': [
          {
            'path': r'C:\tmp\saves\G1R-001.sav',
            'slot': 'G1R-001',
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'a',
            'status': 'ok',
            'playerSaveName': 'Save A',
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await pumpEventQueue();

    // The base recording core has no handler for private.npc.attributes, so it
    // returns the unhandled-command error — which must arrive as an inline
    // error field, not a throw.
    final result = await notifier.loadNpcAttributes('Lizard-1');

    expect(result.attributes, isEmpty);
    expect(result.error, isNotNull);
  });

  test(
    'per-NPC detail cache does not serve the previous save after a slot switch',
    () async {
      // Regression: the per-NPC attribute/inventory memo is keyed by GlobalId.
      // selectedPath moves to the new save at the START of _inspect, but the
      // cache is only invalidated after a SUCCESSFUL inspect — so a load in that
      // window (here: the switch's inspect fails) must NOT return the previous
      // save's memoized future for the same GlobalId.
      final core = _NpcAttrFailSecondInspectCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      await notifier.loadNpcAttributes('Shared-Id'); // memoized for G1R-001
      final before = core.requests
          .where((r) => r.command == 'private.npc.attributes')
          .length;
      expect(before, 1);

      // Switch to another save whose inspect fails: selectedPath becomes
      // G1R-002 but the cache is never invalidated.
      await notifier.inspect(r'C:\tmp\saves\G1R-002.sav');
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');

      await notifier.loadNpcAttributes('Shared-Id');
      final after = core.requests
          .where((r) => r.command == 'private.npc.attributes')
          .length;
      // A fresh request proves the stale G1R-001 memo was dropped, not served.
      expect(after, before + 1);
    },
  );

  // ---------------------------------------------------------------------------
  // Factions (private.factions.list / .forgive)
  // ---------------------------------------------------------------------------

  test('loadFactions sends path and parses the guild list', () async {
    final core = _RecordingCoreService(
      factionsData: {
        'guilds': [
          {
            'guild': 'Guild.Human.OldCamp',
            'label': 'OldCamp',
            'total': 3,
            'forgiven': 1,
            'unforgiven': 2,
          },
          {
            'guild': 'Other',
            'label': 'Other',
            'total': 1,
            'forgiven': 0,
            'unforgiven': 1,
          },
        ],
      },
    );
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadFactions();

    expect(page.error, isNull);
    expect(page.guilds, hasLength(2));
    final oc = page.guilds.first;
    expect(oc.guild, 'Guild.Human.OldCamp');
    expect(oc.label, 'OldCamp');
    expect(oc.total, 3);
    expect(oc.forgiven, 1);
    expect(oc.unforgiven, 2);

    final call = core.requests.lastWhere(
      (r) => r.command == 'private.factions.list',
    );
    expect(call.payload['path'], r'C:\tmp\saves\G1R-001.sav');
  });

  test('loadFactions surfaces a core error inline', () async {
    // No factionsData → the recording core returns the unhandled-command error,
    // which must arrive as an inline error field, not a throw.
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadFactions();

    expect(page.guilds, isEmpty);
    expect(page.error, isNotNull);
  });

  test(
    'setPendingFactionForgive registers a pending edit without an immediate write',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;

      notifier.setPendingFactionForgive('Guild.Human.OldCamp');

      // A draft was registered under the per-guild key — no write fired.
      expect(
        notifier.state.pendingEdits.containsKey(
          'factions.forgive:Guild.Human.OldCamp',
        ),
        isTrue,
      );
      final edit = notifier
          .state
          .pendingEdits['factions.forgive:Guild.Human.OldCamp']!
          .edits
          .single;
      expect(edit['path'], 'private.factions.forgive');
      expect(edit['value'], {'guild': 'Guild.Human.OldCamp'});
      final writesAfter = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      expect(writesAfter, writesBefore);
    },
  );

  test(
    'forgive rides the fixed-size batch (NOT a splicing write) on global save',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingFactionForgive('Guild.Human.OldCamp');
      notifier.setPendingFactionForgive('Guild.Human.NewCamp');

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Both forgives are fixed-size → batched into ONE write_save.
      expect(writes, hasLength(1));
      final edits = writes.single.payload['edits'] as List;
      expect(edits, hasLength(2));
      expect(
        edits.every((e) => (e as Map)['path'] == 'private.factions.forgive'),
        isTrue,
      );
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test('glossary segment target rehydrates from the pending registry', () {
    final notifier = EditorNotifier(
      _RecordingCoreService(),
      saveDir: r'C:\tmp\saves',
    );
    const document = '/Script/Angelscript.Document_Glossary_Meatbug';
    const segment =
        '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Entry2';

    notifier.setPendingGlossarySegment(
      const GlossarySegmentEdit(
        documentClass: document,
        segmentClass: segment,
        unlocked: true,
      ),
    );

    expect(notifier.pendingGlossarySegment(document, segment), isTrue);
    notifier.clearPendingGlossarySegment(document, segment);
    expect(notifier.pendingGlossarySegment(document, segment), isNull);
  });

  test('story changes aggregate case-insensitively and remove on revert', () {
    final notifier = EditorNotifier(
      _RecordingCoreService(),
      saveDir: r'C:\tmp\saves',
    );
    const stone = StoryStateEdit(
      id: 'Stone_OreArmor',
      present: true,
      rawValue: 123,
      expectedStored: true,
      expectedRawValue: 100,
    );
    const chapter = StoryStateEdit(
      id: 'Chapter',
      present: true,
      rawValue: 3,
      expectedStored: false,
      expectedRawValue: null,
    );

    notifier.setStoryStateEdit(stone);
    notifier.setStoryStateEdit(chapter);

    expect(notifier.pendingEditCount, 2);
    expect(notifier.state.pendingEdits.keys, [storyStatePendingKey]);
    expect(notifier.allStoryStateEdits().map((edit) => edit.id), [
      'Chapter',
      'Stone_OreArmor',
    ]);
    expect(notifier.storyStateEditFor(' stone_orearmor '), stone);
    final wire = notifier.pendingEditFor(storyStatePendingKey)!.edits.single;
    expect(wire['path'], storyStateApplyPath);
    expect(((wire['value'] as Map)['changes'] as List), hasLength(2));

    // Same normalized ID, restored to its original snapshot: remove only Stone.
    notifier.setStoryStateEdit(
      const StoryStateEdit(
        id: 'stone_orearmor',
        present: true,
        rawValue: 100,
        expectedStored: true,
        expectedRawValue: 100,
      ),
    );
    expect(notifier.storyStateEditFor('Stone_OreArmor'), isNull);
    expect(notifier.pendingEditCount, 1);

    notifier.clearStoryStateEdit('CHAPTER');
    expect(notifier.pendingEditFor(storyStatePendingKey), isNull);
  });

  test('story apply gets an exclusive structural sub-write', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    notifier.setPendingEdit(
      'publicName',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'After story edit'},
        ],
      ),
    );
    notifier.setStoryStateEdit(
      const StoryStateEdit(
        id: 'Chapter',
        present: true,
        rawValue: 3,
        expectedStored: true,
        expectedRawValue: 2,
      ),
    );

    expect(await notifier.saveAllPending(), isTrue);
    final writes = core.requests
        .where((request) => request.command == 'write_save')
        .toList();
    expect(writes, hasLength(2));
    expect(
      (writes[0].payload['edits'] as List).single['path'],
      'public.m_PlayerSaveName',
    );
    expect(
      (writes[1].payload['edits'] as List).single['path'],
      storyStateApplyPath,
    );
    expect(writes[0].payload['backup'], isTrue);
    expect(writes[1].payload['backup'], isFalse);
  });

  test(
    'story CAS failure refreshes disk and preserves the pending target',
    () async {
      final core = _StoryCasFailureCoreService();
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      await pumpEventQueue();

      final initialPage = await notifier.loadStoryState(includeUnset: true);
      final initialRow = initialPage.values.single;
      expect(initialRow.value, 100);
      notifier.setStoryStateEdit(
        StoryStateEdit.fromValue(initialRow, present: true, rawValue: 123),
      );
      final scansBefore = core.commandCount('scan_save_dir');
      final inspectionsBefore = core.commandCount('inspect_save');
      final backupsBefore = core.commandCount('list_backups');

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('story CAS mismatch'));
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-001.sav');
      expect(core.commandCount('scan_save_dir'), scansBefore + 1);
      expect(core.commandCount('inspect_save'), inspectionsBefore + 1);
      expect(core.commandCount('list_backups'), backupsBefore + 1);
      final writeIndex = core.requests.lastIndexWhere(
        (request) => request.command == 'write_save',
      );
      expect(
        core.requests
            .skip(writeIndex + 1)
            .take(3)
            .map((request) => request.command),
        ['scan_save_dir', 'inspect_save', 'list_backups'],
      );

      final preserved = notifier.storyStateEditFor('Stone_OreArmor');
      expect(preserved, isNotNull);
      expect(preserved!.rawValue, 123);
      expect(preserved.expectedRawValue, 100);

      // The failed CAS fake changes the disk value before returning its error.
      // Re-loading and re-queuing against that row must advance the expected
      // snapshot while retaining the user's desired target.
      final freshPage = await notifier.loadStoryState(includeUnset: true);
      final freshRow = freshPage.values.single;
      expect(freshRow.value, 200);
      notifier.setStoryStateEdit(
        StoryStateEdit.fromValue(freshRow, present: true, rawValue: 123),
      );
      final requeued = notifier.storyStateEditFor('stone_orearmor');
      expect(requeued, isNotNull);
      expect(requeued!.rawValue, 123);
      expect(requeued.expectedRawValue, 200);
    },
  );

  test('generic invalid edits block save and survive NPC actor switching', () {
    final notifier = EditorNotifier(
      _RecordingCoreService(),
      saveDir: r'C:\tmp\saves',
    );
    notifier.setStoryStateEditInvalid(true);

    expect(notifier.state.hasInvalidEdits, isTrue);
    expect(notifier.state.hasUnsavedEdits, isTrue);
    expect(notifier.pendingEditCount, 1);
    expect(notifier.state.hasInvalidNpcEdit, isTrue);

    notifier.setNpcEditInvalid('npc.attributes:Lizard-1');
    notifier.selectActor(
      const Actor.npc(id: 'Lizard-2', name: 'L2', uniqueName: 'Lizard'),
    );
    expect(notifier.state.invalidEditKeys, {storyStatePendingKey});

    notifier.clearAllPendingEdits();
    expect(notifier.state.hasInvalidEdits, isFalse);
    expect(notifier.state.hasUnsavedEdits, isFalse);
  });
}

class _MemoryEditorSettingsStore implements EditorSettingsStore {
  _MemoryEditorSettingsStore([EditorSettings? settings])
    : settings = settings ?? const EditorSettings();

  EditorSettings settings;

  @override
  EditorSettings read() => settings;

  @override
  void write(EditorSettings settings) {
    this.settings = settings;
  }
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

class _RecordingCoreService implements GoresaveCoreService {
  _RecordingCoreService({
    Map<String, Object?>? scanData,
    this.codecCanCompress = true,
    this.typedSearchData,
    this.typedSearchPages,
    this.progressionData,
    this.factionsData,
  }) : scanData = scanData ?? {'saves': <Object?>[]};

  final Map<String, Object?> scanData;
  final bool codecCanCompress;
  final Map<String, Object?>? typedSearchData;

  /// Per-call responses for search_typed_properties (pagination tests). The
  /// n-th search call returns the n-th page; takes precedence over
  /// [typedSearchData]. The last page repeats if called more often.
  final List<Map<String, Object?>>? typedSearchPages;
  var _typedSearchCalls = 0;
  void Function(int call)? onTypedSearchCall;
  Future<void> Function(int call)? waitTypedSearchCall;

  /// Canned response data for query_progression. When null the command falls
  /// through to the default unhandled-command error response.
  final Map<String, Object?>? progressionData;

  /// Canned response data for private.factions.list. When null the command
  /// falls through to the default unhandled-command error response.
  final Map<String, Object?>? factionsData;

  final requests = <_RecordedRequest>[];

  int commandCount(String command) =>
      requests.where((request) => request.command == command).length;

  @override
  String get description => 'recording-core';

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
          'data': {'saveRoot': payload['path'], ...scanData},
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
            'private': {
              'status': preview ? 'decoded_preview' : 'decoded',
              'preview': preview,
              'decodedChunkCount': preview ? 1 : null,
              'totalChunkCount': preview ? 541 : null,
              'strings': preview ? ['Hero'] : ['Hero', 'ChapterOne'],
              'stringCount': preview ? 1 : 2,
              'decompressedSize': 9,
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
                'writable': [
                  'private.player.setPlayerName',
                  'private.profile.setProfileName',
                  'private.player.setAttribute',
                  'private.player.setTransform',
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
            'backups': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav.bak.200',
                'fileName': 'G1R-001.sav.bak.200',
                'fileSize': 914000,
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
      case 'restore_backup':
      case 'restore_deleted_save':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'restoredFrom': payload['backupPath'],
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.300',
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
      case 'write_difficulty':
        final targets = (payload['targets'] as Map?) ?? const {};
        final saveCount = (targets['saves'] as List?)?.length ?? 0;
        final profileCount = targets.containsKey('profile') ? 1 : 0;
        return {
          'ok': true,
          'data': {
            'targetsWritten': saveCount + profileCount,
            'paths': targets['saves'],
          },
        };
      case 'assign_save_profile':
        final profileId = (payload['profileId'] as num).toInt();
        final saves = (scanData['saves'] as List?) ?? <Object?>[];
        final destinationPath = payload['destinationPath'] as String?;
        if (destinationPath != null) {
          final fileName = destinationPath.split(RegExp(r'[\\/]')).last;
          final slot = fileName.toLowerCase().endsWith('.sav')
              ? fileName.substring(0, fileName.length - 4)
              : fileName;
          saves.add({
            'path': destinationPath,
            'slot': slot,
            'format': 'GSAV',
            'fileSize': 100,
            'sha1': 'imported',
            'status': 'ok',
            'playerSaveName': 'Imported save',
            'persistentProfileId': profileId,
          });
        }
        for (final raw in saves.whereType<Map>()) {
          if (raw['path'] == (destinationPath ?? payload['path'])) {
            raw['persistentProfileId'] = profileId;
          }
        }
        return {
          'ok': true,
          'data': {
            'path': destinationPath ?? payload['path'],
            if (destinationPath != null) 'sourcePath': payload['path'],
            'persistentPath': payload['persistentPath'],
            'profileId': profileId,
            'bytesChanged': true,
            'backupPath': r'C:\tmp\saves\G1R-006.sav.bak.1',
            'persistentBackupPath':
                r'C:\tmp\saves\PersistentDataList.sav.bak.1',
          },
        };
      case 'remove_save_from_profile':
        final profileId = (payload['profileId'] as num).toInt();
        final slot = payload['slot'] as String;
        final saves = (scanData['saves'] as List?) ?? <Object?>[];
        saves.removeWhere(
          (raw) =>
              raw is Map && raw['slot'] == slot && raw['status'] == 'missing',
        );
        for (final raw in saves.whereType<Map>()) {
          if (raw['slot'] == slot) raw.remove('persistentProfileId');
        }
        for (final raw
            in ((scanData['profiles'] as List?) ?? <Object?>[])
                .whereType<Map>()) {
          for (final key in ['savedSlots', 'quickSaveSlots', 'autoSaveSlots']) {
            (raw[key] as List?)?.removeWhere((value) => value == slot);
          }
        }
        return {
          'ok': true,
          'data': {
            'slot': slot,
            'persistentPath': payload['persistentPath'],
            'profileId': profileId,
            'bytesChanged': true,
            'backupPath': null,
            'persistentBackupPath':
                r'C:\tmp\saves\PersistentDataList.sav.bak.2',
          },
        };
      case 'delete_save':
        final profileId = (payload['profileId'] as num).toInt();
        final slot = payload['slot'] as String;
        final saves = (scanData['saves'] as List?) ?? <Object?>[];
        saves.removeWhere((raw) => raw is Map && raw['slot'] == slot);
        for (final raw
            in ((scanData['profiles'] as List?) ?? <Object?>[])
                .whereType<Map>()) {
          for (final key in ['savedSlots', 'quickSaveSlots', 'autoSaveSlots']) {
            (raw[key] as List?)?.removeWhere((value) => value == slot);
          }
        }
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'slot': slot,
            'persistentPath': payload['persistentPath'],
            'profileId': profileId,
            'backupPath': r'C:\tmp\saves\goresave_backups\G1R-006.sav.bak.2',
            'persistentBackupPath':
                r'C:\tmp\saves\goresave_backups\PersistentDataList.sav.bak.2',
            'persistentPostDeleteSha1': 'post-delete-profile-sha',
            'deletedSaveSha1': 'deleted-save-sha',
            'deletedPersistentSha1': 'deleted-persistent-sha',
          },
        };
      case 'search_typed_properties':
        final pages = typedSearchPages;
        if (pages != null && pages.isNotEmpty) {
          final call = _typedSearchCalls++;
          final page = pages[call.clamp(0, pages.length - 1)];
          onTypedSearchCall?.call(call);
          await waitTypedSearchCall?.call(call);
          return {'ok': true, 'data': page};
        }
        return {
          'ok': true,
          'data':
              typedSearchData ??
              {
                'query': '',
                'offset': 0,
                'limit': 1000,
                'total': 0,
                'count': 0,
                'results': [],
              },
        };
      case 'validate_codec_roundtrip':
        return {
          'ok': true,
          'data': {
            'status': 'codec_roundtrip_passed',
            'chunkIndex': 0,
            'decompressedSize': 131072,
            'recompressedSize': 1759,
          },
        };
      case 'check_codec':
        return {
          'ok': true,
          'data': {
            'backend': 'kraken',
            'available': true,
            'canDecompress': true,
            'canCompress': codecCanCompress,
            'status': codecCanCompress ? 'ready' : 'decode_only',
            'details': {'adapter': 'kraken'},
          },
        };
      case 'query_progression':
        if (progressionData != null) {
          return {'ok': true, 'data': progressionData!};
        }
        return {
          'ok': false,
          'error': {'message': 'Unhandled command $command'},
        };
      case 'private.factions.list':
        if (factionsData != null) {
          return {'ok': true, 'data': factionsData!};
        }
        return {
          'ok': false,
          'error': {'message': 'Unhandled command $command'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled command $command'},
        };
    }
  }
}

class _RejectExternalInspectCoreService extends _RecordingCoreService {
  _RejectExternalInspectCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'inspect_save' &&
        (payload['path'] as String?)?.startsWith(r'D:\archive') == true) {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {'message': 'invalid external file'},
      };
    }
    return super.execute(command, payload: payload);
  }
}

class _FailingAssignProfileCoreService extends _RecordingCoreService {
  _FailingAssignProfileCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'assign_save_profile') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {'message': 'external import failed'},
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// write_save always fails.
class _FailingWriteCoreService extends _RecordingCoreService {
  _FailingWriteCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_save') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {'message': 'write failed'},
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Mimics an optimistic-concurrency failure where another writer changes the
/// sole story value before the core rejects our stale expectedRawValue.
class _StoryCasFailureCoreService extends _RecordingCoreService {
  _StoryCasFailureCoreService()
    : super(
        scanData: {
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 914367,
              'sha1': 'abc',
              'status': 'ok',
              'playerSaveName': 'Auto',
            },
          ],
        },
      );

  var storyRawValue = 100;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'query_progression' && payload['section'] == 'story') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'section': 'story',
          'total': 1,
          'storedTotal': 1,
          'catalogTotal': 470,
          'unsetTotal': 469,
          'unknownStoredTotal': 0,
          'offset': payload['offset'] ?? 0,
          'limit': payload['limit'] ?? 1000,
          'count': 1,
          'entries': [
            {
              'id': 'Stone_OreArmor',
              'rawValue': storyRawValue,
              'stored': true,
              'catalogKnown': true,
              'path': ['StoryPropertyValues', '{Stone_OreArmor}'],
              'semanticType': 'timeMarker',
              'declaredType': 'FInGameTime',
            },
          ],
          'currentGameTimeSeconds': 300.0,
          'writable': true,
        },
      };
    }
    if (command == 'write_save') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      storyRawValue = 200;
      return {
        'ok': false,
        'error': {
          'code': 'story_value_conflict',
          'message': 'story CAS mismatch: Stone_OreArmor changed on disk',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Succeeds the write but reports that the placement sidecar could not be
/// written — the shape the core uses when the save is good and only the undo
/// note failed.
class _PlacementWarningCoreService extends _RecordingCoreService {
  _PlacementWarningCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_save') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1',
          'placementNoteWarning':
              r'C:\tmp\saves\goresave_backups\npc_placements.json is not '
              'readable NPC-placement JSON',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Restores a backup successfully but reports that the placement notes that
/// describe it could not be installed.
class _RestoreWarningCoreService extends _RecordingCoreService {
  _RestoreWarningCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'restore_backup') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'restoredFrom': r'C:\tmp\saves\G1R-001.sav.bak.1',
          'placementNoteWarning':
              r'C:\tmp\saves\goresave_backups\npc_placements.json is not '
              'readable NPC-placement JSON',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Commits the first write_save with a placement warning, then fails the
/// second. The move is on disk either way, so the warning has to outlive the
/// later failure.
class _WarnThenFailCoreService extends _RecordingCoreService {
  _WarnThenFailCoreService({super.scanData});

  var _writes = 0;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_save') {
      _writes++;
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      if (_writes >= 2) {
        return {
          'ok': false,
          'error': {'message': 'write failed'},
        };
      }
      return {
        'ok': true,
        'data': {
          'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1',
          'placementNoteWarning':
              r'C:\tmp\saves\goresave_backups\npc_placements.json is not '
              'readable NPC-placement JSON',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Succeeds the first write_save, fails the second. Used to prove that
/// saveAllPending clears only the snapshot keys whose sub-write committed.
class _FailSecondWriteCoreService extends _RecordingCoreService {
  _FailSecondWriteCoreService({super.scanData});

  var _writes = 0;
  var refreshScans = 0;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      refreshScans++;
    }
    if (command == 'write_save') {
      _writes++;
      if (_writes >= 2) {
        requests.add(
          _RecordedRequest(command, Map<String, Object?>.from(payload)),
        );
        return {
          'ok': false,
          'error': {'message': 'second write failed'},
        };
      }
    }
    return super.execute(command, payload: payload);
  }
}

/// Succeeds the first write_save, then throws like a failed worker/native FFI
/// call. This must enter the same partial-commit convergence path as an
/// `ok: false` response because the first write has already changed disk.
class _ThrowSecondWriteCoreService extends _RecordingCoreService {
  _ThrowSecondWriteCoreService({super.scanData});

  var _writes = 0;
  var refreshScans = 0;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') refreshScans++;
    if (command == 'write_save') {
      _writes++;
      if (_writes >= 2) {
        requests.add(
          _RecordedRequest(command, Map<String, Object?>.from(payload)),
        );
        throw Exception('native worker died');
      }
    }
    return super.execute(command, payload: payload);
  }
}

class _FailingScanCoreService extends _RecordingCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {'message': 'save folder is unavailable'},
      };
    }
    return super.execute(command, payload: payload);
  }
}

class _ThrowingScanCoreService extends _RecordingCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      throw Exception('native scan failed');
    }
    return super.execute(command, payload: payload);
  }
}

/// write_save throws (as the persistent worker isolate does via
/// CoreWorkerException on a native failure) instead of returning `ok: false`.
class _ThrowingWriteCoreService extends _RecordingCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_save') {
      throw Exception('native worker died');
    }
    return super.execute(command, payload: payload);
  }
}

/// write_save completes only after [gate] resolves.
class _SlowWriteCoreService extends _RecordingCoreService {
  _SlowWriteCoreService(this.gate);

  final Future<void> gate;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_save') {
      await gate;
    }
    return super.execute(command, payload: payload);
  }
}

/// Codec decodes but the verification round-trip fails (e.g. a mis-resolved
/// encoder on an unknown build).
class _FailingSecondInspectCoreService extends _RecordingCoreService {
  var _inspectCalls = 0;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'inspect_save') {
      _inspectCalls++;
      if (_inspectCalls > 1) {
        requests.add(
          _RecordedRequest(command, Map<String, Object?>.from(payload)),
        );
        return {
          'ok': false,
          'error': {'message': 'private payload decode failed'},
        };
      }
    }
    return super.execute(command, payload: payload);
  }
}

class _FailingVerifyCoreService extends _RecordingCoreService {
  _FailingVerifyCoreService() : super(codecCanCompress: false);

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'validate_codec_roundtrip') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {
          'message': 'codec roundtrip output did not match decoded chunk',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Returns a canned `private.npc.attributes` response (one Health row with full
/// typed Base/Current paths), mirroring the core contract.
class _NpcAttributesCoreService extends _RecordingCoreService {
  _NpcAttributesCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.npc.attributes') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      const base = [
        'm_GenericData',
        '{CharacterStates}',
        'AnyCharacterType',
        'AttributeSetsByClass',
        '{/Script/G1R.AttributeSet_Health}',
        'Attributes',
        '{Health}',
      ];
      return {
        'ok': true,
        'data': {
          'attributes': [
            {
              'key': 'Health',
              'base': 25.6,
              'current': 25.6,
              'basePath': [...base, 'BaseValue'],
              'currentPath': [...base, 'CurrentValue'],
            },
          ],
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Like [_NpcAttributesCoreService] but the SECOND `inspect_save` fails. Used to
/// reproduce the slot-switch window where `selectedPath` has already moved to the
/// new save but the per-NPC cache was never invalidated (invalidate runs only on
/// a successful inspect).
class _NpcAttrFailSecondInspectCoreService extends _NpcAttributesCoreService {
  var _inspects = 0;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'inspect_save') {
      _inspects++;
      if (_inspects >= 2) {
        return {
          'ok': false,
          'error': {'message': 'inspect failed'},
        };
      }
    }
    return super.execute(command, payload: payload);
  }
}

/// Serves `private.npc.list` as a PAGED endpoint that clamps `limit` to
/// [pageSize] (mirroring the core's 1000-cap), so `loadAllNpcActors` must page
/// to collect all [total] NPCs.
class _PagedNpcCoreService extends _RecordingCoreService {
  _PagedNpcCoreService({required this.total, required this.pageSize});

  final int total;
  final int pageSize;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.npc.list') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      final offset = (payload['offset'] as num?)?.toInt() ?? 0;
      final requested = (payload['limit'] as num?)?.toInt() ?? 100;
      // Mimic the core's clamp(1, 1000) on the page size.
      final limit = requested.clamp(1, pageSize);
      final start = offset.clamp(0, total);
      final end = (start + limit).clamp(0, total);
      final npcs = [
        for (var i = start; i < end; i++) {'id': 'Npc-$i', 'isDead': false},
      ];
      return {
        'ok': true,
        'data': {
          'npcs': npcs,
          'total': total,
          'offset': offset,
          'limit': limit,
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Serves `private.characters.list` with the save's own "Hero" ACTOR row (as
/// real saves carry — see the gore-save `characters_list` integration test)
/// plus a normal NPC and a knowledge-only orphan. [failList] flips the command
/// to an error response, proving an error page leaves the stashed hero
/// GlobalId untouched.
class _CharactersListCoreService extends _RecordingCoreService {
  var failList = false;

  /// Runs right after a `private.characters.list` call is recorded and before
  /// its response is returned — lets a test switch saves mid-fetch to prove
  /// the hero stash is pinned to the path the request was issued against.
  void Function()? onListCall;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.characters.list') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      onListCall?.call();
      if (failList) {
        return {
          'ok': false,
          'error': {'message': 'characters list failed'},
        };
      }
      return {
        'ok': true,
        'data': {
          'total': 3,
          'characters': [
            {
              'globalId': 'Hero',
              'uniqueName': 'Hero',
              'isDead': false,
              'hasInventory': false,
              'hasKnowledge': true,
              'hasEvents': true,
            },
            {
              'globalId': 'Lizard-WP_A',
              'uniqueName': 'Lizard',
              'isDead': false,
              'hasInventory': true,
              'hasKnowledge': false,
              'hasEvents': false,
            },
            {
              'globalId': null,
              'uniqueName': 'ST_VLK_Mud_Sleeper',
              'isDead': false,
              'hasInventory': false,
              'hasKnowledge': true,
              'hasEvents': false,
            },
          ],
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// Like [_PagedNpcCoreService] but runs [onFirstListPage] right after the FIRST
/// `private.npc.list` page is built (and recorded), letting a test switch saves
/// mid-fetch to prove the paging loop pins its starting path.
class _MidFetchSwitchNpcCoreService extends _PagedNpcCoreService {
  _MidFetchSwitchNpcCoreService({
    required super.total,
    required super.pageSize,
  });

  void Function()? onFirstListPage;
  bool _fired = false;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.npc.list' && !_fired) {
      _fired = true;
      final result = await super.execute(command, payload: payload);
      // Fire (do NOT await — awaiting a re-entrant core call here would
      // deadlock on the notifier's serialized core queue).
      onFirstListPage?.call();
      return result;
    }
    return super.execute(command, payload: payload);
  }
}
