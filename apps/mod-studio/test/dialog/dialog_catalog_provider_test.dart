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

  group('buildDialogRows with onlyIds', () {
    const catalog = <String, Map<String, String>>{
      'info_aaron_001': {'de_A': 'Hallo'},
      'info_aaron_002': {'de_A': 'Tschau'},
      'info_bob_001': {'de_A': 'Moin'},
      'gvl_zed_001': {'de_A': 'Weg da'},
    };

    test('restricts before grouping: only filtered lines, groups, counts', () {
      final rows = buildDialogRows(
        catalog,
        onlyIds: {'info_aaron_002', 'gvl_zed_001'},
      );
      final ids = rows.whereType<DialogLineRow>().map((l) => l.id).toList();
      expect(ids, ['info_aaron_002', 'gvl_zed_001']);
      final groups = rows.whereType<DialogGroupRow>().toList();
      // Bob has no filtered line, so his group is gone entirely; Aaron's
      // count reflects the filtered lines (1), not the catalog total (2).
      expect(
        groups.map((g) => '${g.isBark}:${g.speaker}:${g.lineCount}').toList(),
        ['false:aaron:1', 'true:zed:1'],
      );
    });

    test('empty set yields no rows', () {
      expect(buildDialogRows(catalog, onlyIds: const {}), isEmpty);
    });

    test('ids absent from the catalog are ignored', () {
      final rows = buildDialogRows(
        catalog,
        onlyIds: {'info_bob_001', 'info_ghost_999'},
      );
      expect(rows.whereType<DialogLineRow>().single.id, 'info_bob_001');
      expect(rows.whereType<DialogGroupRow>().single.lineCount, 1);
    });

    test('null onlyIds behaves like the unfiltered call', () {
      final unfiltered = buildDialogRows(catalog);
      final explicitNull = buildDialogRows(catalog, onlyIds: null);
      expect(
        explicitNull.whereType<DialogLineRow>().map((l) => l.id).toList(),
        unfiltered.whereType<DialogLineRow>().map((l) => l.id).toList(),
      );
      expect(unfiltered.whereType<DialogLineRow>(), hasLength(4));
    });
  });
}
