import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('SaveSlot uses player save name as display name', () {
    final slot = SaveSlot.fromJson({
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'format': 'GSAV',
      'fileSize': 12,
      'sha1': 'abc',
      'status': 'ok',
      'playerSaveName': 'Chapter 1',
      'persistentPlayerSaveName': 'Persistent Chapter 1',
      'chapterId': 2,
      'mapName': 'OldCamp',
      'timePlayedSeconds': 3661.5,
      'quickSave': true,
      'autoSave': false,
      'screenshot': {
        'mimeType': 'image/jpeg',
        'byteLength': 6,
        'bytesBase64': '/9gBAv/Z',
      },
    });

    expect(slot.displayName, 'Chapter 1');
    expect(slot.chunkCount, isNull);
    expect(slot.persistentPlayerSaveName, 'Persistent Chapter 1');
    expect(slot.chapterId, 2);
    expect(slot.mapName, 'OldCamp');
    expect(slot.timePlayedSeconds, 3661.5);
    expect(slot.quickSave, isTrue);
    expect(slot.autoSave, isFalse);
    expect(slot.screenshot?.mimeType, 'image/jpeg');
    expect(slot.screenshot?.byteLength, 6);
    expect(slot.screenshot?.bytesBase64, '/9gBAv/Z');
  });

  test('ProfileSummary reads slot groups and difficulty flags', () {
    final profile = ProfileSummary.fromJson({
      'profileId': 0,
      'profileName': '0',
      'quickSaveSlots': ['G1R-001', 'G1R-002', 'G1R-003'],
      'autoSaveSlots': ['G1R-001', 'G1R-002'],
      'savedSlots': ['G1R-001', 'G1R-002'],
      'difficultyPreset': '/Game/Difficulty/Normal',
      'customCombatSettings': '/Game/Difficulty/Combat',
      'customResourcesSettings': '/Game/Difficulty/Resources',
      'customProgressionSettings': '/Game/Difficulty/Progression',
      'survival': false,
      'permanentDeath': true,
      'permanentDeathGameOver': true,
      'fakeSloppyCombos': false,
      'maxQuick': 3,
      'maxAuto': 2,
    });

    expect(profile.profileId, 0);
    expect(profile.displayName, 'Profile 0');
    expect(profile.quickSaveSlots, ['G1R-001', 'G1R-002', 'G1R-003']);
    expect(profile.autoSaveSlots, ['G1R-001', 'G1R-002']);
    expect(profile.savedSlots, ['G1R-001', 'G1R-002']);
    expect(profile.difficultyPreset, '/Game/Difficulty/Normal');
    expect(profile.permanentDeath, isTrue);
    expect(profile.maxQuick, 3);
  });

  test('SaveInspection reads nested public and stream summaries', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'trailerSize': 44,
      'screenshot': {
        'mimeType': 'image/jpeg',
        'byteLength': 6,
        'bytesBase64': '/9gBAv/Z',
      },
      'public': {'slotName': 'G1R-001', 'playerSaveName': 'Colony'},
      'persistent': {
        'playerSaveName': 'Colony, Day 1',
        'chapterId': 1,
        'mapName': 'MainMap',
        'timePlayedSeconds': 6963.25,
        'timeLoadedSeconds': 0.0,
        'quickSave': false,
        'autoSave': true,
        'profileId': 0,
      },
      'compressedStream': {
        'method': 'Oodle',
        'chunkCount': 3,
        'uncompressedSize': 131072,
      },
      'private': {
        'status': 'native_encoder_in_progress',
        'message': 'Native encoder is unavailable',
      },
    });

    expect(inspection.playerSaveName, 'Colony');
    expect(inspection.persistentPlayerSaveName, 'Colony, Day 1');
    expect(inspection.chapterId, 1);
    expect(inspection.mapName, 'MainMap');
    expect(inspection.timePlayedSeconds, 6963.25);
    expect(inspection.timeLoadedSeconds, 0.0);
    expect(inspection.quickSave, isFalse);
    expect(inspection.autoSave, isTrue);
    expect(inspection.persistentProfileId, 0);
    expect(inspection.compressionMethod, 'Oodle');
    expect(inspection.chunkCount, 3);
    expect(inspection.trailerSize, 44);
    expect(inspection.privateDecoded, isFalse);
    expect(inspection.privateStatus, 'native_encoder_in_progress');
    expect(inspection.privateDecompressedSize, isNull);
    expect(inspection.privateStringCount, isNull);
    expect(inspection.screenshot?.bytesBase64, '/9gBAv/Z');
  });

  test('SaveInspection reads decoded private strings', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded',
        'decompressedSize': 24,
        'stringCount': 2,
        'strings': ['Hero', 'ChapterOne'],
      },
    });

    expect(inspection.privateDecoded, isTrue);
    expect(inspection.privateDecompressedSize, 24);
    expect(inspection.privateStringCount, 2);
    expect(inspection.privateStrings, ['Hero', 'ChapterOne']);
  });

  test('SaveInspection reads typed parse status', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded',
        'typedParse': {
          'status': 'ok',
          'rootClass': '/Script/Angelscript.GothicFinalDataGame',
          'propertyCount': 889357,
          'maxDepth': 10,
          'consumed': 76866251,
          'payloadSize': 76866251,
        },
      },
    });

    expect(inspection.privateTypedParseStatus, 'ok');
    expect(inspection.privateTypedVerified, isTrue);
    expect(inspection.privateTypedPropertyCount, 889357);
    expect(inspection.privateTypedMaxDepth, 10);
  });

  test('SaveInspection treats failed or missing typed parse as unverified', () {
    final failed = SaveInspection.fromJson({
      'format': 'GSAV',
      'size': 1,
      'sha1': 'a',
      'private': {
        'status': 'decoded',
        'typedParse': {'status': 'failed', 'message': 'boom'},
      },
    });
    expect(failed.privateTypedParseStatus, 'failed');
    expect(failed.privateTypedVerified, isFalse);

    final missing = SaveInspection.fromJson({
      'format': 'GSAV',
      'size': 1,
      'sha1': 'a',
      'private': {'status': 'decoded'},
    });
    expect(missing.privateTypedParseStatus, isNull);
    expect(missing.privateTypedVerified, isFalse);
  });

  test('SaveInspection reads typed private player summary', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded',
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
      },
    });

    expect(inspection.privatePlayer.saveVersionNumber, 17);
    expect(inspection.privatePlayer.currentWorld, 'WORLD');
    expect(inspection.privatePlayer.playerName, 'Hero');
    expect(inspection.privatePlayer.profileName, '0');
    expect(inspection.privatePlayer.transform?.location.x, 10.0);
    expect(inspection.privatePlayer.transform?.location.y, 20.0);
    expect(inspection.privatePlayer.transform?.location.z, 30.0);
    expect(inspection.privatePlayer.transform?.rotation.pitch, 40.0);
    expect(inspection.privatePlayer.transform?.rotation.yaw, 50.0);
    expect(inspection.privatePlayer.transform?.rotation.roll, 60.0);
    expect(inspection.privatePlayer.attributes.first.id, 'Health');
    expect(inspection.privatePlayer.attributes.first.baseValue, 40.0);
    expect(inspection.privatePlayer.attributes.first.currentValue, 25.0);
    expect(inspection.privatePlayer.attributes.last.id, 'Strength');
    expect(inspection.privatePlayer.scriptPaths, [
      '/Script/Angelscript.GothicFinalDataGame',
    ]);
    expect(inspection.privatePlayer.properties, [
      'm_SaveVersionNumber',
      'm_CurrentWorld',
    ]);
    expect(inspection.privatePlayer.writable, [
      'private.player.setPlayerName',
      'private.profile.setProfileName',
      'private.player.setAttribute',
      'private.player.setTransform',
    ]);
  });

  test('SaveInspection reads typed private inventory summary', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded',
        'inventory': {
          'candidateCount': 2,
          'candidates': [
            'ITMI_GOLD',
            '/Game/G1R/Items/BP_Item_Ore.BP_Item_Ore_C',
          ],
          'itemStackCount': 1,
          'itemScope': 'player_inventory_region',
          'items': [
            {
              'id': 'ItMi_Orenugget',
              'path': '/Script/Angelscript.ItMi_Orenugget',
              'count': 42,
            },
          ],
          'scriptPaths': ['/Script/G1R.InventorySaveGameData'],
          'properties': ['m_InventoryItems', 'm_StackCount'],
          'writable': ['private.inventory.setItemCount'],
        },
      },
    });

    expect(inspection.privateInventory.candidateCount, 2);
    expect(inspection.privateInventory.candidates, [
      'ITMI_GOLD',
      '/Game/G1R/Items/BP_Item_Ore.BP_Item_Ore_C',
    ]);
    expect(inspection.privateInventory.itemStackCount, 1);
    expect(inspection.privateInventory.itemScope, 'player_inventory_region');
    expect(inspection.privateInventory.items.single.id, 'ItMi_Orenugget');
    expect(
      inspection.privateInventory.items.single.path,
      '/Script/Angelscript.ItMi_Orenugget',
    );
    expect(inspection.privateInventory.items.single.count, 42);
    expect(inspection.privateInventory.scriptPaths, [
      '/Script/G1R.InventorySaveGameData',
    ]);
    expect(inspection.privateInventory.properties, [
      'm_InventoryItems',
      'm_StackCount',
    ]);
    expect(inspection.privateInventory.writable, [
      'private.inventory.setItemCount',
    ]);
  });

  test('SaveInspection reads typed private progression summary', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'slot': 'G1R-001',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded',
        'progression': {
          'candidateCount': 3,
          'candidates': [
            'Quest.Main.Chapter01',
            'Dialog.Diego.IntroDone',
            'Knowledge.OldCamp.PathKnown',
          ],
          'gameplayTags': ['Quest.Main.Chapter01', 'Dialog.Diego.IntroDone'],
          'sections': ['Generated events', 'Memorized events'],
          'scriptPaths': ['/Script/G1R.QuestSaveGameData'],
          'properties': ['m_GeneratedEvents', 'm_MemorizedEvents'],
        },
      },
    });

    expect(inspection.privateProgression.candidateCount, 3);
    expect(inspection.privateProgression.candidates, [
      'Quest.Main.Chapter01',
      'Dialog.Diego.IntroDone',
      'Knowledge.OldCamp.PathKnown',
    ]);
    expect(inspection.privateProgression.gameplayTags, [
      'Quest.Main.Chapter01',
      'Dialog.Diego.IntroDone',
    ]);
    expect(inspection.privateProgression.sections, [
      'Generated events',
      'Memorized events',
    ]);
    expect(inspection.privateProgression.scriptPaths, [
      '/Script/G1R.QuestSaveGameData',
    ]);
    expect(inspection.privateProgression.properties, [
      'm_GeneratedEvents',
      'm_MemorizedEvents',
    ]);
  });

  test('SaveInspection treats decoded preview as usable private data', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'size': 1024,
      'sha1': 'abc',
      'private': {
        'status': 'decoded_preview',
        'preview': true,
        'decodedChunkCount': 1,
        'totalChunkCount': 541,
        'strings': ['Hero'],
      },
    });

    expect(inspection.privateDecoded, isTrue);
    expect(inspection.privatePreview, isTrue);
    expect(inspection.privateDecodedChunkCount, 1);
    expect(inspection.privateTotalChunkCount, 541);
  });

  test('SaveInspection treats decoded_preview status as preview without flag', () {
    final inspection = SaveInspection.fromJson({
      'format': 'GSAV',
      'path': r'C:\saves\G1R-001.sav',
      'size': 1024,
      'sha1': 'abc',
      // status says preview but the explicit preview flag is absent.
      'private': {'status': 'decoded_preview', 'strings': ['Hero']},
    });

    expect(inspection.privateDecoded, isTrue);
    expect(inspection.privatePreview, isTrue);
    expect(inspection.privateEditable, isFalse);
  });

  test('CodecStatus exposes native adapter without DLL path fields', () {
    final codec = CodecStatus.fromJson({
      'available': false,
      'canDecompress': false,
      'canCompress': false,
      'status': 'native_encoder_in_progress',
      'adapter': 'pure_rust_kraken',
      'message': 'Native encoder is unavailable',
    });

    expect(codec.available, isFalse);
    expect(codec.status, 'native_encoder_in_progress');
    expect(codec.adapter, 'pure_rust_kraken');
    expect(codec.canDecompress, isFalse);
    expect(codec.canCompress, isFalse);
  });

  test('BackupEntry reads validated backup metadata', () {
    final backup = BackupEntry.fromJson({
      'path': r'C:\saves\G1R-001.sav.bak.200',
      'fileName': 'G1R-001.sav.bak.200',
      'fileSize': 4096,
      'sha1': 'abc123',
      'createdEpoch': 200,
      'status': 'ok',
      'playerSaveName': 'Before edit',
    });

    expect(backup.path, r'C:\saves\G1R-001.sav.bak.200');
    expect(backup.fileName, 'G1R-001.sav.bak.200');
    expect(backup.fileSize, 4096);
    expect(backup.createdEpoch, 200);
    expect(backup.status, 'ok');
    expect(backup.playerSaveName, 'Before edit');
    expect(backup.canRestore, isTrue);
  });

  test('BackupEntry reads companion scope and disables direct restore', () {
    final backup = BackupEntry.fromJson({
      'path': r'C:\saves\PersistentDataList.sav.bak.250',
      'fileName': 'PersistentDataList.sav.bak.250',
      'fileSize': 8192,
      'sha1': 'def456',
      'createdEpoch': 250,
      'status': 'ok',
      'scope': 'persistent_data_list',
      'slotName': 'G1R-001',
      'playerSaveName': 'Before companion edit',
    });

    expect(backup.scope, 'persistent_data_list');
    expect(backup.slotName, 'G1R-001');
    expect(backup.playerSaveName, 'Before companion edit');
    expect(backup.canRestore, isFalse);
  });
}
