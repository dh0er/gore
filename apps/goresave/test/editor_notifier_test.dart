import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';

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

  test(
    'writePlayerSaveName sends length-changing public metadata edit with slot-list sync',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      await notifier.writePlayerSaveName('Much Longer Save Name');

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['backup'], isTrue);
      expect(write.payload['syncPersistentDataList'], isTrue);
      expect(write.payload['edits'], [
        {'path': 'public.m_PlayerSaveName', 'value': 'Much Longer Save Name'},
      ]);
      expect(
        notifier.state.lastWriteMessage,
        contains(r'PersistentDataList.sav.bak.2'),
      );
    },
  );

  test('writePrivateFString sends host-backed private edit', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    await notifier.writePrivateFString(oldValue: 'Hero', newValue: 'Mage');

    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(write.payload['binaryHost'], {
      'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      'exePath':
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    });
    expect(write.payload['edits'], [
      {
        'path': 'private.replaceFString',
        'value': {'oldValue': 'Hero', 'newValue': 'Mage'},
      },
    ]);
  });

  test('writePrivatePlayerName sends structured host-backed player edit', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    await notifier.writePrivatePlayerName('Nameless');

    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(write.payload['binaryHost'], {
      'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      'exePath':
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    });
    expect(write.payload['edits'], [
      {'path': 'private.player.setPlayerName', 'value': 'Nameless'},
    ]);
  });

  test(
    'writePrivateProfileName sends structured host-backed profile edit',
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

      await notifier.writePrivateProfileName('goresave');

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(write.payload['edits'], [
        {'path': 'private.profile.setProfileName', 'value': 'goresave'},
      ]);
    },
  );

  test(
    'writePlayerAttribute sends structured host-backed attribute edit',
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

      await notifier.writePlayerAttribute(
        id: 'Health',
        baseValue: 77,
        currentValue: 66,
      );

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(write.payload['edits'], [
        {
          'path': 'private.player.setAttribute',
          'value': {'id': 'Health', 'baseValue': 77.0, 'currentValue': 66.0},
        },
      ]);
    },
  );

  test(
    'writePlayerTransform sends structured host-backed transform edit',
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

      await notifier.writePlayerTransform(
        locationX: 100,
        locationY: 200,
        locationZ: 300,
        rotationPitch: 1,
        rotationYaw: 2,
        rotationRoll: 3,
      );

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(write.payload['edits'], [
        {
          'path': 'private.player.setTransform',
          'value': {
            'location': {'x': 100.0, 'y': 200.0, 'z': 300.0},
            'rotation': {'pitch': 1.0, 'yaw': 2.0, 'roll': 3.0},
          },
        },
      ]);
    },
  );

  test(
    'writeInventoryItemCount sends host-backed private inventory edit',
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

      await notifier.writeInventoryItemCount(
        id: 'ItMi_Orenugget',
        path: '/Script/Angelscript.ItMi_Orenugget',
        count: 99,
      );

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(write.payload['edits'], [
        {
          'path': 'private.inventory.setItemCount',
          'value': {
            'id': 'ItMi_Orenugget',
            'path': '/Script/Angelscript.ItMi_Orenugget',
            'count': 99,
          },
        },
      ]);
    },
  );

  test(
    'writeInventoryItemCounts sends one host-backed write with multiple inventory edits',
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

      await notifier.writeInventoryItemCounts([
        const InventoryItemCountChange(
          id: 'ItMi_Orenugget',
          path: '/Script/Angelscript.ItMi_Orenugget',
          count: 99,
        ),
        const InventoryItemCountChange(
          id: 'ItFo_Cheese',
          path: '/Script/Angelscript.ItFo_Cheese',
          count: 7,
        ),
      ]);

      final write = core.requests.lastWhere(
        (request) => request.command == 'write_save',
      );
      expect(write.payload['binaryHost'], {
        'helperPath': r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
        'exePath':
            r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      });
      expect(write.payload['edits'], [
        {
          'path': 'private.inventory.setItemCount',
          'value': {
            'id': 'ItMi_Orenugget',
            'path': '/Script/Angelscript.ItMi_Orenugget',
            'count': 99,
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
      expect(
        notifier.state.lastWriteMessage,
        contains('2 inventory counts saved'),
      );
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
  _RecordingCoreService({Map<String, Object?>? scanData})
    : scanData = scanData ?? {'saves': <Object?>[]};

  final Map<String, Object?> scanData;
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
            'canCompress': true,
            'status': 'codec_host_ready',
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
