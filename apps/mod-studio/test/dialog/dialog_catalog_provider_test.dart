import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dialog/domain/dialog_catalog_provider.dart';

void main() {
  group('buildDialogRows', () {
    test('line rows carry bark flag matching their group', () {
      final rows = buildDialogRows({
        'info_aaron_001': {'de_A': 'Hallo'},
        'gvl_aaron_002': {'de_A': 'Weg da'},
      });
      final lines = rows.whereType<DialogLineRow>().toList();
      expect(lines.singleWhere((l) => l.id == 'info_aaron_001').isBark, false);
      expect(lines.singleWhere((l) => l.id == 'gvl_aaron_002').isBark, true);
    });

    test('conversation groups precede bark groups, speakers alphabetical', () {
      final rows = buildDialogRows({
        'gvl_zed_001': {'de_A': 'z'},
        'info_bob_001': {'de_A': 'b'},
        'dia_alice_002': {'de_A': 'a2'},
        'dia_alice_001': {'de_A': 'a1'},
        'text_menu_001': {'de_A': 'excluded'},
      });
      final groups = rows.whereType<DialogGroupRow>().toList();
      expect(
        groups.map((g) => '${g.isBark}:${g.speaker}').toList(),
        ['false:alice', 'false:bob', 'true:zed'],
      );
      expect(groups.first.lineCount, 2);
      // Lines are id-sorted within their group.
      final aliceIds = rows
          .whereType<DialogLineRow>()
          .where((l) => l.speaker == 'alice')
          .map((l) => l.id)
          .toList();
      expect(aliceIds, ['dia_alice_001', 'dia_alice_002']);
    });

    test('group rows and their line rows share groupKey', () {
      final rows = buildDialogRows({
        'info_aaron_001': {'de_A': 'Hallo'},
        'info_aaron_002': {'de_A': 'Tschau'},
        'gvl_aaron_001': {'de_A': 'Weg da'},
      });
      DialogGroupRow? current;
      for (final row in rows) {
        switch (row) {
          case DialogGroupRow():
            current = row;
          case DialogLineRow():
            expect(row.groupKey, current!.groupKey);
        }
      }
    });

    test('same speaker in info_ and gvl_ ids forms two separate groups', () {
      final rows = buildDialogRows({
        'info_aaron_001': {'de_A': 'Hallo'},
        'gvl_aaron_001': {'de_A': 'Weg da'},
      });
      final groups = rows.whereType<DialogGroupRow>().toList();
      expect(groups, hasLength(2));
      expect(groups.map((g) => g.groupKey).toSet(), hasLength(2));
      expect(groups.every((g) => g.speaker == 'aaron'), true);
    });

    test('svm_ prefix is grouped as bark', () {
      final rows = buildDialogRows({
        'svm_guard_001': {'de_A': 'Halt!'},
      });
      final group = rows.whereType<DialogGroupRow>().single;
      expect(group.isBark, true);
      expect(group.speaker, 'guard');
      expect(rows.whereType<DialogLineRow>().single.isBark, true);
    });

    test('id without a second underscore uses the remainder as speaker', () {
      final rows = buildDialogRows({
        'gvl_guard': {'de_A': 'Halt!'},
      });
      final group = rows.whereType<DialogGroupRow>().single;
      expect(group.speaker, 'guard');
      final line = rows.whereType<DialogLineRow>().single;
      expect(line.speaker, 'guard');
      expect(line.groupKey, group.groupKey);
    });
  });
}
