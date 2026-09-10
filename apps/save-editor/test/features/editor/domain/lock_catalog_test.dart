import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/lock_catalog.dart';

void main() {
  group('difficulty bars', () {
    // The game's lockpicking widget holds four Difficulty_1..4 images and
    // fills each when the lock's raw tier reaches its own threshold. The
    // thresholds are 1, 2, 4, 6, decompiled from the widget's SetLockDifficulty
    // bytecode — so the seven internal tiers collapse onto four bars and the
    // player never sees the raw number.
    test('the seven tiers collapse onto the game\'s four bars', () {
      expect(lockDifficultyBars(1), 1);
      expect(lockDifficultyBars(2), 2);
      expect(lockDifficultyBars(3), 2);
      expect(lockDifficultyBars(4), 3);
      expect(lockDifficultyBars(5), 3);
      expect(lockDifficultyBars(6), 4);
      expect(lockDifficultyBars(7), 4);
    });

    test('never exceeds the four pips the widget draws', () {
      for (var tier = 1; tier <= 12; tier++) {
        expect(lockDifficultyBars(tier), lessThanOrEqualTo(lockDifficultyBarCount));
      }
    });
  });

  group('catalog parsing', () {
    test('reads a lock with every field', () {
      final catalog = LockCatalog.fromJsonString('''
{"version":1,"locks":[
  {"n":"IO_OC_CHEST_GOMEZ_01","k":"chest","a":"OC","d":7,
   "l":"OC_Chest_Gomez_01_Lock","keys":["ItKe_Gomez_01"],"r":true}
]}
''');
      final lock = catalog.locks.single;
      expect(lock.name, 'IO_OC_CHEST_GOMEZ_01');
      expect(lock.kind, LockKind.chest);
      expect(lock.area, 'OC');
      expect(lock.difficulty, 7);
      expect(lock.lockId, 'OC_Chest_Gomez_01_Lock');
      expect(lock.keys, ['ItKe_Gomez_01']);
      expect(lock.randomized, isTrue);
    });

    test('a door without a difficulty keeps its keys', () {
      final catalog = LockCatalog.fromJsonString('''
{"version":1,"locks":[
  {"n":"AbandonedMine_Fence_Door","k":"door","a":"AM","keys":["ItKe_AbandonedMine_01"]}
]}
''');
      final lock = catalog.locks.single;
      expect(lock.kind, LockKind.door);
      expect(lock.difficulty, isNull, reason: 'key-only locks are not pickable');
      expect(lock.keys, ['ItKe_AbandonedMine_01']);
    });

    test('lookup folds case, like the FName set in a save', () {
      final catalog = LockCatalog.fromJsonString(
        '{"version":1,"locks":[{"n":"IO_OC_CHEST_DEXTER","k":"chest","a":"OC"}]}',
      );
      expect(catalog.byName('io_oc_chest_dexter'), isNotNull);
      expect(catalog.byName('nope'), isNull);
    });

    test('the search string covers the name and its keys', () {
      final catalog = LockCatalog.fromJsonString('''
{"version":1,"locks":[
  {"n":"IO_OM_CHEST_AARON","k":"chest","a":"OM","keys":["ItKe_Om_02"]}
]}
''');
      final search = catalog.locks.single.search;
      expect(search, contains('aaron'));
      expect(search, contains('itke_om_02'));
    });
  });
}
