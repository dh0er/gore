import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';

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
              'value': {'id': 'Health', 'baseValue': 77.0, 'currentValue': 66.0},
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
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
      );
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
              'value': {'id': 'Health', 'baseValue': 80.0, 'currentValue': 80.0},
            },
          ],
        ),
      );

      await notifier.saveAllPending();

      final write = core.requests.lastWhere(
        (r) => r.command == 'write_save',
      );
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
            'value': {'path': ['MaxHealth'], 'value': 99.0},
          },
        ],
      ),
    );
    expect(notifier.state.pendingEdits.length, 2);

    // Toolbar Refresh — same selected path stays selected.
    await notifier.refresh();

    expect(notifier.state.pendingEdits, isEmpty,
        reason: 'refresh() must clear ALL pending edits');
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

    expect(notifier.state.pendingEdits, isEmpty,
        reason: 'restoreBackup() must clear pending edits via refresh()');
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
            'value': {'path': ['MaxHealth'], 'value': 99.0},
          },
          {
            'path': 'private.typed.setValue',
            'value': {'path': ['Strength'], 'value': 20.0},
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

  test('loadHeroAttributes pages through results beyond the search cap',
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
  });

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
  }) : scanData = scanData ?? {'saves': <Object?>[]};

  final Map<String, Object?> scanData;
  final bool codecCanCompress;
  final Map<String, Object?>? typedSearchData;

  /// Per-call responses for search_typed_properties (pagination tests). The
  /// n-th search call returns the n-th page; takes precedence over
  /// [typedSearchData]. The last page repeats if called more often.
  final List<Map<String, Object?>>? typedSearchPages;
  var _typedSearchCalls = 0;
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
      case 'search_typed_properties':
        final pages = typedSearchPages;
        if (pages != null && pages.isNotEmpty) {
          final page = pages[_typedSearchCalls.clamp(0, pages.length - 1)];
          _typedSearchCalls++;
          return {'ok': true, 'data': page};
        }
        return {
          'ok': true,
          'data': typedSearchData ??
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
      requests.add(_RecordedRequest(command, Map<String, Object?>.from(payload)));
      return {
        'ok': false,
        'error': {'message': 'write failed'},
      };
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
class _FailingVerifyCoreService extends _RecordingCoreService {
  _FailingVerifyCoreService() : super(codecCanCompress: false);

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'validate_codec_roundtrip') {
      requests.add(_RecordedRequest(command, Map<String, Object?>.from(payload)));
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
