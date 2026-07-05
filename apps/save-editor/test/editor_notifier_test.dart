import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';

void main() {
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
      final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');

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
    'inspect sends no codec config and decodes all chunks',
    () async {
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
    },
  );

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

  test(
    'saveAllPending issues ONE write_save with mixed edits in stable key order',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
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

      // Two splicing edits → two sequential writes. Keys sort so 'npc.revive:A'
      // (first write, commits) precedes 'npc.revive:B' (second write, fails).
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
      notifier.setPendingEdit(
        'npc.revive:B',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'B'},
            },
          ],
        ),
      );

      final scansBefore = core.refreshScans;
      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      expect(notifier.state.error, isNotNull);
      // First write committed → its key is cleared; the failed second's key stays.
      expect(notifier.state.pendingEdits.containsKey('npc.revive:A'), isFalse);
      expect(notifier.state.pendingEdits.containsKey('npc.revive:B'), isTrue);
      // The committed edit changed the file, so the panes must be refreshed from
      // disk even though a later sub-write failed. refresh() begins with a
      // scan_save_dir, so exactly one ADDITIONAL scan proves the partial-commit
      // refresh ran (vs. the old early-return that left the UI stale).
      expect(core.refreshScans, scansBefore + 1);
    },
  );

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
      notifier.setPendingEdit(
        'npc.revive:B',
        const PendingSaveEdit(
          edits: [
            {
              'path': 'private.npc.revive',
              'value': {'id': 'B'},
            },
          ],
        ),
      );

      final ok = await notifier.saveAllPending();

      expect(ok, isFalse);
      // Slot switched to the only remaining save…
      expect(notifier.state.selectedPath, r'C:\tmp\saves\G1R-002.sav');
      // …so the uncommitted edit was dropped, NOT re-targeted at G1R-002.
      expect(notifier.state.pendingEdits.containsKey('npc.revive:B'), isFalse);
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
    'saveAllPending splits a splicing edit and a typed edit into separate writes',
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
      // No "must be saved on its own" guard fires anymore — the split replaces it.
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // Two writes: the splicing removeItem on its own + the typed setValue batch.
      expect(writes, hasLength(2));
      // Each splicing edit is its own single-edit write.
      final splicing = writes.firstWhere(
        (w) => (w.payload['edits'] as List).any(
          (e) => (e as Map)['path'] == 'private.inventory.removeItem',
        ),
      );
      expect(splicing.payload['edits'], hasLength(1));
      // The typed edit lands in its own (fixed) batch with no splicing peer.
      final fixed = writes.firstWhere(
        (w) => (w.payload['edits'] as List).any(
          (e) => (e as Map)['path'] == 'private.typed.setValue',
        ),
      );
      expect(
        (fixed.payload['edits'] as List).every(
          (e) => (e as Map)['path'] == 'private.typed.setValue',
        ),
        isTrue,
      );
      // All pending cleared after success.
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test(
    'saveAllPending splits a mixed batch: revive alone, addItem alone, '
    'setValue batched — with backup only on the first write',
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
      expect(notifier.state.error, isNull);
      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      // revive alone + addItem alone + the fixed setValue batch = three writes.
      expect(writes, hasLength(3));

      bool writeHas(_RecordedRequest w, String path) =>
          (w.payload['edits'] as List).any(
            (e) => (e as Map)['path'] == path,
          );

      final reviveWrite = writes.firstWhere(
        (w) => writeHas(w, 'private.npc.revive'),
      );
      expect(reviveWrite.payload['edits'], hasLength(1));
      final addItemWrite = writes.firstWhere(
        (w) => writeHas(w, 'private.inventory.addItem'),
      );
      expect(addItemWrite.payload['edits'], hasLength(1));
      final fixedWrite = writes.firstWhere(
        (w) => writeHas(w, 'private.player.setAttribute'),
      );
      expect(fixedWrite.payload['edits'], hasLength(1));

      // Backup-once: exactly one write carries backup:true (the first), the
      // rest backup:false — one pristine snapshot per Save.
      final backupTrue = writes.where((w) => w.payload['backup'] == true);
      expect(backupTrue, hasLength(1));
      expect(writes.first.payload['backup'], isTrue);
      for (final w in writes.skip(1)) {
        expect(w.payload['backup'], isFalse);
      }
      expect(notifier.state.pendingEdits, isEmpty);
    },
  );

  test(
    'saveAllPending issues two writes for two distinct splicing edits',
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
      // Two separate single-edit writes — never batched together.
      expect(writes, hasLength(2));
      for (final w in writes) {
        expect(w.payload['edits'], hasLength(1));
      }
      // Backup on the first write only.
      expect(writes.first.payload['backup'], isTrue);
      expect(writes.last.payload['backup'], isFalse);
    },
  );

  test(
    'saveAllPending puts syncPersistentDataList on the fixed-batch write only',
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
      expect(writes, hasLength(2));
      // The fixed batch (public name) carries the sync flag; the splicing
      // revive write must not.
      final fixed = writes.firstWhere(
        (w) => (w.payload['edits'] as List).any(
          (e) => (e as Map)['path'] == 'public.m_PlayerSaveName',
        ),
      );
      expect(fixed.payload['syncPersistentDataList'], isTrue);
      final splicing = writes.firstWhere(
        (w) => (w.payload['edits'] as List).any(
          (e) => (e as Map)['path'] == 'private.npc.revive',
        ),
      );
      expect(splicing.payload.containsKey('syncPersistentDataList'), isFalse);
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
      expect(
        core.requests.where((r) => r.command == 'write_save'),
        isEmpty,
      );
      expect(notifier.state.error, contains('EffectSpec'));
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

  // ---------------------------------------------------------------------------
  // Bug #1: a fresh inspection must reset the selected actor to the player so a
  // stale NPC GlobalId from the previous save can't drive the actor-aware tabs.
  // ---------------------------------------------------------------------------
  test('inspecting a new save resets the selected actor to the player', () async {
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
  });

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
      expect(writes, hasLength(2));
      bool writeHas(_RecordedRequest w, String path) =>
          (w.payload['edits'] as List).any((e) => (e as Map)['path'] == path);
      final fixedIndex = writes.indexWhere(
        (w) => writeHas(w, 'private.typed.setValue'),
      );
      final reviveIndex = writes.indexWhere(
        (w) => writeHas(w, 'private.npc.revive'),
      );
      // The fixed Health batch is issued BEFORE the revive write, so revive's
      // HP (the last write to the NPC's HP) is final on disk.
      expect(fixedIndex, lessThan(reviveIndex));
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
      expect(writes, hasLength(2));
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
  test('loadAllNpcActors pins the save path across a mid-fetch save switch', () async {
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
  });

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

  test('validateCodecRoundtrip sends no codec config and reports success',
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
  });

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
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
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
    // All progression loaders share _queryProgression; loadKnowledgeEntries is
    // the exercised representative.
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\tmp\saves',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final page = await notifier.loadKnowledgeEntries('OC_STT_Diego');

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
    'applyAddKnowledgeCharacter issues one write_save with the addCharacter edit',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      // Whitespace is trimmed before it reaches the core.
      final result = await notifier.applyAddKnowledgeCharacter('  NewNpc  ');

      expect(result, isTrue);

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      expect(write.payload['backup'], isTrue);
      final edits = write.payload['edits'] as List;
      expect(edits, hasLength(1));
      final edit = edits.single as Map;
      expect(edit['path'], 'private.knowledge.addCharacter');
      expect(edit['value'], {'value': 'NewNpc'});
    },
  );

  test(
    'applyAddKnowledgeCharacter is blocked when pendingEdits is non-empty',
    () async {
      final core = _RecordingCoreService();
      final notifier = EditorNotifier(
        core,
        saveDir: r'C:\tmp\saves',
      );
      await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

      notifier.setPendingEdit(
        'x',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': 'Draft'},
          ],
        ),
      );

      final writesBefore = core.requests
          .where((r) => r.command == 'write_save')
          .length;

      final result = await notifier.applyAddKnowledgeCharacter('NewNpc');

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

  /// Canned response data for query_progression. When null the command falls
  /// through to the default unhandled-command error response.
  final Map<String, Object?>? progressionData;

  /// Canned response data for private.factions.list. When null the command
  /// falls through to the default unhandled-command error response.
  final Map<String, Object?>? factionsData;

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
        for (var i = start; i < end; i++)
          {'id': 'Npc-$i', 'isDead': false},
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
  _MidFetchSwitchNpcCoreService({required super.total, required super.pageSize});

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
