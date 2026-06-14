import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';

void main() {
  test('uses persisted editor paths before defaults', () {
    final core = _RecordingCoreService();
    final store = _MemoryEditorSettingsStore(
      const EditorSettings(
        saveDir: r'D:\G1R\Saves',
        codecHostPath: r'D:\Tools\goresave\goresave_g1r_codec_host.exe',
        gameExePath:
            r'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      ),
    );

    final notifier = EditorNotifier(core, settingsStore: store);

    expect(notifier.state.saveDir, r'D:\G1R\Saves');
    expect(
      notifier.state.codecHostPath,
      r'D:\Tools\goresave\goresave_g1r_codec_host.exe',
    );
    expect(
      notifier.state.gameExePath,
      r'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
  });

  test('path setters persist editor settings', () async {
    final core = _RecordingCoreService();
    final store = _MemoryEditorSettingsStore();
    final notifier = EditorNotifier(core, settingsStore: store);

    await notifier.setSaveDir(r'E:\G1R\Saved\SaveGames');
    await notifier.setCodecHostPath(r'E:\goresave\goresave_g1r_codec_host.exe');
    await notifier.setGameExePath(
      r'E:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );

    expect(store.settings.saveDir, r'E:\G1R\Saved\SaveGames');
    expect(
      store.settings.codecHostPath,
      r'E:\goresave\goresave_g1r_codec_host.exe',
    );
    expect(
      store.settings.gameExePath,
      r'E:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
  });

  test('checkCodec sends configured binary host paths', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );

    await notifier.checkCodec();

    final checkCodec = core.requests.lastWhere(
      (request) => request.command == 'check_codec',
    );
    expect(checkCodec.payload['binaryHost'], {
      'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      'exePath':
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    });
  });

  test(
    'refresh parses profiles, screenshots, and sends scan codec host',
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
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );

      await pumpEventQueue();

      final scan = core.requests.firstWhere(
        (request) => request.command == 'scan_save_dir',
      );
      expect(scan.payload['binaryHost'], {
        'helperPath': r'C:\tools\goresave_g1r_codec_host.exe',
        'exePath': r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      });
      expect(notifier.state.profiles.single.displayName, 'Profile 0');
      expect(notifier.state.activeProfile?.profileId, 0);
      expect(notifier.state.selectedSave?.screenshot?.byteLength, 6);
    },
  );

  test(
    'inspect sends configured binary host paths and decodes all chunks',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
        codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        gameExePath:
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      );

      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      final inspect = core.requests.lastWhere(
        (request) => request.command == 'inspect_save',
      );
      expect(inspect.payload.containsKey('privateChunkLimit'), isFalse);
      expect(inspect.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(notifier.state.backups.single.fileName, 'G1R-001.sav.bak.200');
      expect(notifier.state.backups.single.playerSaveName, 'Before edit');
      expect(
        notifier.state.companionBackups.single.fileName,
        'PersistentDataList.sav.bak.250',
      );
      expect(notifier.state.companionBackups.single.canRestore, isFalse);
    },
  );

  test('restoreBackup sends backup path and refreshes selected save', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
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

  test(
    'saveAllPending issues ONE write_save with mixed edits in stable key order',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
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

  // ---------------------------------------------------------------------------
  // Pending difficulty edit folded into the global Save/Reset flow
  // ---------------------------------------------------------------------------

  /// A scan with one save attributed to profile 7, so selectedSave and
  /// editedSaveProfile resolve.
  Map<String, Object?> difficultyScanData() => {
    'saves': [
      {
        'path': r'C:\tmp\saves\G1R-001.sav',
        'slot': 'G1R-001',
        'format': 'GSAV',
        'fileSize': 100,
        'sha1': 'a',
        'status': 'ok',
        'playerSaveName': 'Save A',
        'persistentProfileId': 7,
      },
      {
        'path': r'C:\tmp\saves\G1R-002.sav',
        'slot': 'G1R-002',
        'format': 'GSAV',
        'fileSize': 100,
        'sha1': 'b',
        'status': 'ok',
        'playerSaveName': 'Save B',
        'persistentProfileId': 7,
      },
    ],
    'profiles': [
      {
        'profileId': 7,
        'profileName': 'Nameless Hero',
        'quickSaveSlots': <String>[],
        'autoSaveSlots': <String>[],
        'savedSlots': ['G1R-001', 'G1R-002'],
      },
    ],
    'activeProfileId': 7,
  };

  EditorNotifier difficultyNotifier(_RecordingCoreService core) {
    return EditorNotifier(
      core,
      saveDir: r'C:\tmp\saves',
      codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
      gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
    );
  }

  test(
    'pending difficulty edit counts in pendingEditCount/hasUnsavedEdits and '
    'enables the global Save',
    () async {
      final core = _RecordingCoreService(scanData: difficultyScanData());
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();

      expect(notifier.state.pendingEditCount, 0);
      expect(notifier.state.hasUnsavedEdits, isFalse);

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {
            'preset': 'Hard',
            'flowHelper': false,
            'permadeath': true,
          },
        ),
      );

      // The global badge counts the difficulty edit as one unsaved change.
      expect(notifier.state.pendingEditCount, 1);
      expect(notifier.state.hasUnsavedEdits, isTrue);
    },
  );

  test(
    'global saveAllPending dispatches write_difficulty and clears the edit',
    () async {
      final core = _RecordingCoreService(scanData: difficultyScanData());
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      // Select the first save so it is the edited save.
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {
            'preset': 'Hard',
            'flowHelper': false,
            'permadeath': true,
          },
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      final writes = core.requests
          .where((r) => r.command == 'write_difficulty')
          .toList();
      expect(writes, hasLength(1), reason: 'exactly one write_difficulty');
      final payload = writes.single.payload;
      expect(payload['difficulty'], {
        'preset': 'Hard',
        'flowHelper': false,
        'permadeath': true,
      });
      // Only the current save targeted (no propagation), profile not written.
      final targets = payload['targets'] as Map;
      expect(targets['saves'], [r'C:\tmp\saves\G1R-001.sav']);
      expect(targets.containsKey('profile'), isFalse);
      expect(payload['backup'], isTrue);
      // Codec host attached (private-payload edit).
      expect(payload['binaryHost'], isNotNull);
      // Edit cleared after success.
      expect(notifier.state.pendingDifficulty, isNull);
      expect(notifier.state.hasUnsavedEdits, isFalse);
    },
  );

  test(
    'global saveAllPending performs BOTH write_save and write_difficulty with '
    'a single refresh',
    () async {
      final core = _RecordingCoreService(scanData: difficultyScanData());
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      final scansBefore = core.requests
          .where((r) => r.command == 'scan_save_dir')
          .length;

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Renamed'},
          ],
        ),
      );
      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {'preset': 'Gothic', 'flowHelper': true, 'permadeath': false},
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isTrue);
      // Both core writes issued, write_save before write_difficulty.
      final writeCommands = core.requests
          .map((r) => r.command)
          .where((c) => c == 'write_save' || c == 'write_difficulty')
          .toList();
      expect(writeCommands, ['write_save', 'write_difficulty']);
      // Exactly ONE refresh (scan_save_dir) at the end.
      final scansAfter = core.requests
          .where((r) => r.command == 'scan_save_dir')
          .length;
      expect(scansAfter - scansBefore, 1, reason: 'refresh runs once');
      // Everything cleared.
      expect(notifier.state.pendingEdits, isEmpty);
      expect(notifier.state.pendingDifficulty, isNull);
    },
  );

  test(
    'partial commit converges: write_save succeeds, write_difficulty fails — '
    'slot edits cleared (committed), difficulty kept, honest error',
    () async {
      final core = _FailingDifficultyWriteCoreService(
        scanData: difficultyScanData(),
      );
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Renamed'},
          ],
        ),
      );
      const pendingDifficulty = PendingDifficulty(
        difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
      );
      notifier.setPendingDifficulty(pendingDifficulty);

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse, reason: 'overall save failed (difficulty did not land)');
      // write_save ran (and succeeded) before the failing write_difficulty.
      final writeCommands = core.requests
          .map((r) => r.command)
          .where((c) => c == 'write_save' || c == 'write_difficulty')
          .toList();
      expect(writeCommands, ['write_save', 'write_difficulty']);
      // The committed slot edits are now on disk — they MUST NOT stay pending
      // (that would be the divergence bug + a double-apply on retry).
      expect(
        notifier.state.pendingEdits.containsKey('publicName'),
        isFalse,
        reason: 'committed slot edits cleared regardless of difficulty outcome',
      );
      // The difficulty did NOT land — keep ONLY it pending so the user can
      // retry just the difficulty.
      expect(notifier.state.pendingDifficulty, isNotNull);
      expect(notifier.state.pendingDifficulty, pendingDifficulty);
      // Honest, specific error surfaced.
      expect(
        notifier.state.error,
        contains('Slot changes saved, but difficulty failed'),
      );
      expect(notifier.state.error, contains('difficulty write failed'));
    },
  );

  test(
    'thrown write_difficulty converges: write_save succeeds, write_difficulty '
    'THROWS — slot edits cleared, difficulty KEPT (not discarded), error surfaced',
    () async {
      final core = _ThrowingDifficultyWriteCoreService(
        scanData: difficultyScanData(),
      );
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Renamed'},
          ],
        ),
      );
      const pendingDifficulty = PendingDifficulty(
        difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
      );
      notifier.setPendingDifficulty(pendingDifficulty);

      final ok = await notifier.saveAllPending();

      expect(
        ok,
        isFalse,
        reason: 'overall save failed (difficulty threw, did not land)',
      );
      // write_save ran (and succeeded) before the throwing write_difficulty.
      final writeCommands = core.requests
          .map((r) => r.command)
          .where((c) => c == 'write_save' || c == 'write_difficulty')
          .toList();
      expect(writeCommands, ['write_save', 'write_difficulty']);
      // Committed slot edits are on disk — they MUST NOT stay pending.
      expect(
        notifier.state.pendingEdits.containsKey('publicName'),
        isFalse,
        reason: 'committed slot edits cleared (converged with disk)',
      );
      // The thrown difficulty write never landed — it MUST be retained, not
      // discarded by clearPendingDifficulty().
      expect(
        notifier.state.pendingDifficulty,
        pendingDifficulty,
        reason: 'a thrown difficulty write keeps the pending edit',
      );
      // An honest error is surfaced (not silently swallowed).
      expect(notifier.state.error, isNotNull);
      expect(
        notifier.state.error,
        contains('Slot changes saved, but difficulty failed'),
      );
    },
  );

  test(
    'global Reset (refresh) clears a pending difficulty edit',
    () async {
      final core = _RecordingCoreService(scanData: difficultyScanData());
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
        ),
      );
      expect(notifier.state.pendingDifficulty, isNotNull);

      // Global Reset re-scans/re-inspects, which clears all pending edits.
      await notifier.refresh();

      expect(notifier.state.pendingDifficulty, isNull);
      expect(notifier.state.hasUnsavedEdits, isFalse);
    },
  );

  test(
    'difficulty propagation binds to the EDITED SAVE profile (allSaves + '
    'profile targets resolved from selectedSave)',
    () async {
      final core = _RecordingCoreService(scanData: difficultyScanData());
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
          alsoProfile: true,
          allSaves: true,
        ),
      );

      final ok = await notifier.saveAllPending();
      expect(ok, isTrue);

      final write = core.requests.lastWhere(
        (r) => r.command == 'write_difficulty',
      );
      final targets = write.payload['targets'] as Map;
      // allSaves → every save of profile 7 (both G1R-001 and G1R-002).
      expect(
        (targets['saves'] as List).toSet(),
        {r'C:\tmp\saves\G1R-001.sav', r'C:\tmp\saves\G1R-002.sav'},
      );
      // alsoProfile → PersistentDataList.sav next to the current save, profile 7.
      final profileTarget = targets['profile'] as Map;
      expect(profileTarget['path'], r'C:\tmp\saves\PersistentDataList.sav');
      expect(profileTarget['profileId'], 7);
    },
  );

  test(
    'global saveAllPending surfaces the codec error when difficulty is pending '
    'but the codec is not compress-ready',
    () async {
      final core = _RecordingCoreService(
        scanData: difficultyScanData(),
        codecCanCompress: false,
      );
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, contains('verified G1R codec host'));
      // No write was issued.
      expect(
        core.requests.where((r) => r.command == 'write_difficulty'),
        isEmpty,
      );
      // The pending edit is kept so the user can verify the codec and retry.
      expect(notifier.state.pendingDifficulty, isNotNull);
    },
  );

  test(
    'global saveAllPending fails loudly and keeps the pending difficulty when '
    'propagation is requested but the edited save profile cannot be resolved',
    () async {
      // A scan where the selected save\'s persistentProfileId (99) matches no
      // profile in state.profiles, so editedSaveProfile is null — propagation
      // cannot bind to a profile.
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
              'persistentProfileId': 99,
            },
          ],
          'profiles': [
            {
              'profileId': 7,
              'profileName': 'Nameless Hero',
              'quickSaveSlots': <String>[],
              'autoSaveSlots': <String>[],
              'savedSlots': ['G1R-001'],
            },
          ],
          'activeProfileId': 7,
        },
      );
      final notifier = difficultyNotifier(core);
      await pumpEventQueue();
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Sanity: the edited save\'s profile is genuinely unresolvable.
      expect(notifier.state.editedSaveProfile, isNull);

      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {'preset': 'Hard', 'flowHelper': false, 'permadeath': true},
          alsoProfile: true,
        ),
      );

      final ok = await notifier.saveAllPending();

      // Fails loudly with a clear error.
      expect(ok, isFalse);
      expect(notifier.state.error, contains('could not be resolved'));
      // No write was issued — the current-slot save must not slip through.
      expect(
        core.requests.where((r) => r.command == 'write_difficulty'),
        isEmpty,
      );
      // The pending edit is KEPT so nothing is silently lost.
      expect(notifier.state.pendingDifficulty, isNotNull);
      expect(notifier.state.pendingDifficulty!.alsoProfile, isTrue);
    },
  );

  test('saveAllPending keeps pending edits on failure', () async {
    final core = _FailingWriteCoreService();
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
    // Pending edits must be preserved so the user can retry.
    expect(notifier.state.pendingEdits.containsKey('publicName'), isTrue);
  });

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
    'saveAllPending refuses a structural inventory edit batched with typed edits',
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

      expect(ok, isFalse);
      expect(notifier.state.error, contains('saved on its own'));
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
      // Both pending entries survive so the user can save them separately.
      expect(notifier.state.pendingEdits.length, 2);
    },
  );

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

  test('verifyCodec unlocks compress edits for an unverified build', () async {
    final core = _RecordingCoreService(codecCanCompress: false);
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await Future<void>.delayed(Duration.zero);

    // Codec decodes but is not auto-trusted for compression yet.
    expect(notifier.state.codecCompressReady, isFalse);
    expect(notifier.state.codecNeedsVerification, isTrue);

    await notifier.verifyCodec();

    final verify = core.requests.lastWhere(
      (request) => request.command == 'validate_codec_roundtrip',
    );
    expect(verify.payload['path'], r'C:\tmp\saves\G1R-001.sav');
    expect(verify.payload['binaryHost'], isNotNull);
    expect(notifier.state.codecVerified, isTrue);
    expect(notifier.state.codecCompressReady, isTrue);
    expect(notifier.state.codecNeedsVerification, isFalse);
  });

  test('verifyCodec surfaces failure and keeps edits locked', () async {
    final core = _FailingVerifyCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
    await Future<void>.delayed(Duration.zero);

    await notifier.verifyCodec();

    expect(notifier.state.codecVerified, isFalse);
    expect(notifier.state.codecCompressReady, isFalse);
    expect(notifier.state.codecError, contains('roundtrip'));
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
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
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
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
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

  test(
    'validateCodecRoundtrip sends selected save and binary host paths',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
        codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        gameExePath:
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      await notifier.validateCodecRoundtrip();

      final validate = core.requests.lastWhere(
        (request) => request.command == 'validate_codec_roundtrip',
      );
      expect(validate.payload['path'], r'C:\tmp\saves\G1R-001.sav');
      expect(validate.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
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
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\tmp\saves',
      codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
      gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
    );
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
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
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

  test('progression loaders surface core errors inline', () async {
    // The default _RecordingCoreService returns ok:false for query_progression
    // (no progressionData set), so the loader should surface the error inline.
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\tmp\saves',
      codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
      gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadKnowledgeCharacters();

    expect(page.error, isNotNull);
  });

  test(
    'applyMemoryEventEdit is blocked and sets error when isLoading is true',
    () async {
      // Use a slow write to hold the notifier in isLoading state, then verify
      // that a concurrent applyMemoryEventEdit sets a user-visible error.
      final gate = Completer<void>();
      final core = _SlowWriteCoreService(gate.future);
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
      notifier.setPendingEdit(
        'x',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Slow'},
          ],
        ),
      );

      // Start a write that will stall — notifier is now isLoading.
      final writeFuture = notifier.saveAllPending();
      expect(notifier.state.isLoading, isTrue);

      // applyMemoryEventEdit must refuse and set an error while loading.
      final result = await notifier.applyMemoryEventEdit(
        MemoryEventEdit.remove(arrayPath: const ['MemorizedEvents'], index: 0),
      );

      expect(result, isFalse);
      expect(notifier.state.error, isNotNull);
      expect(
        notifier.state.error,
        contains('Another operation is in progress'),
      );

      // Unblock the write so the test can cleanly complete.
      gate.complete();
      await writeFuture;
    },
  );

  test(
    'applyMemoryEventEdit is blocked and sets error when pendingEdits is non-empty',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Seed a pending edit (e.g. an unsaved quest-state change).
      notifier.setPendingEdit(
        'x',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.typed.setValue',
              'value': {
                'path': ['CurrentState'],
                'value': 'EQuestState::None',
              },
            },
          ],
        ),
      );

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;

      final result = await notifier.applyMemoryEventEdit(
        MemoryEventEdit.remove(arrayPath: const ['MemorizedEvents'], index: 0),
      );

      expect(result, isFalse);
      expect(notifier.state.error, isNotNull);
      // No write_save must have been issued.
      final writesAfter = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      expect(writesAfter, writesBefore);
      // Pending edit must still be intact.
      expect(notifier.state.pendingEdits.containsKey('x'), isTrue);
    },
  );

  test(
    'applyMemoryEventEdit is blocked and preserves a pending difficulty edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
        codecHostPath: r'C:\tools\goresave_g1r_codec_host.exe',
        gameExePath: r'C:\Games\G1R\G1R-Win64-Shipping.exe',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Seed an unsaved difficulty edit (lives in pendingDifficulty, NOT
      // pendingEdits) — a memory-event write would otherwise refresh and
      // silently discard it.
      notifier.setPendingDifficulty(
        const PendingDifficulty(
          difficulty: {
            'preset': 'Hard',
            'flowHelper': false,
            'permadeath': true,
          },
        ),
      );
      expect(notifier.state.hasUnsavedEdits, isTrue);

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;

      final result = await notifier.applyMemoryEventEdit(
        MemoryEventEdit.remove(arrayPath: const ['MemorizedEvents'], index: 0),
      );

      expect(result, isFalse);
      expect(notifier.state.error, isNotNull);
      // No write_save must have been issued.
      final writesAfter = core.requests
          .where((r) => r.command == 'write_save')
          .length;
      expect(writesAfter, writesBefore);
      // The pending difficulty edit must still be intact (not silently dropped).
      expect(notifier.state.pendingDifficulty, isNotNull);
    },
  );

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
  }) : scanData = scanData ?? {'saves': <Object?>[]};

  final Map<String, Object?> scanData;
  final bool codecCanCompress;
  final Map<String, Object?>? typedSearchData;

  /// Per-call responses for search_typed_properties (pagination tests). The
  /// n-th search call returns the n-th page; takes precedence over
  /// [typedSearchData]. The last page repeats if called more often.
  final List<Map<String, Object?>>? typedSearchPages;
  var _typedSearchCalls = 0;

  /// Canned response data for query_progression. When null the command falls
  /// through to the default unhandled-command error response.
  final Map<String, Object?>? progressionData;

  final requests = <_RecordedRequest>[];

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
      case 'search_typed_properties':
        final pages = typedSearchPages;
        if (pages != null && pages.isNotEmpty) {
          final page = pages[_typedSearchCalls.clamp(0, pages.length - 1)];
          _typedSearchCalls++;
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
            'selectedBackend': 'g1r_binary_host',
            'available': true,
            'canDecompress': true,
            'canCompress': codecCanCompress,
            'status': codecCanCompress
                ? 'codec_host_ready'
                : 'codec_host_supported_needs_runtime_selftest',
            'adapter': 'g1r_binary_host',
            'message': 'G1R codec host is configured.',
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
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled command $command'},
        };
    }
  }
}

/// write_save always fails.
class _FailingWriteCoreService extends _RecordingCoreService {
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

/// write_save succeeds, but write_difficulty always fails — used to exercise
/// the partial-commit convergence path (slot edits land on disk, difficulty
/// does not).
class _FailingDifficultyWriteCoreService extends _RecordingCoreService {
  _FailingDifficultyWriteCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_difficulty') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': false,
        'error': {'message': 'difficulty write failed'},
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// write_save succeeds, but write_difficulty THROWS (e.g. malformed native
/// JSON from the core) instead of returning an error result — exercises the
/// thrown-write convergence path (the difficulty edit must be KEPT pending).
class _ThrowingDifficultyWriteCoreService extends _RecordingCoreService {
  _ThrowingDifficultyWriteCoreService({super.scanData});

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'write_difficulty') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      throw const FormatException('malformed native difficulty response');
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
