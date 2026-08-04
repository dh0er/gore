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
      expect(groups.map((g) => '${g.isBark}:${g.speaker}').toList(), [
        'false:alice',
        'false:bob',
        'true:zed',
      ]);
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

    test('actor-qualified Asghan and Viper lines use speaker names', () {
      final rows = buildDialogRows({
        'grd_263_asghan_open_info_06_02': {'de_A': 'Asghan'},
        'stt_302_viper_greet_info_11_02': {'de_A': 'Viper'},
        'grd_armor_bot_h_01': {'de_A': 'not dialog'},
      });

      expect(rows.whereType<DialogGroupRow>().map((group) => group.speaker), [
        'asghan',
        'viper',
      ]);
      expect(rows.whereType<DialogLineRow>().map((line) => line.id), [
        'grd_263_asghan_open_info_06_02',
        'stt_302_viper_greet_info_11_02',
      ]);
    });

    test('historic psi-qualified mission lines use actual speaker', () {
      final rows = buildDialogRows({
        'mis_1_psi_kalom_success_10_01': {'de_A': 'Kalom'},
        'sit_2_psi_yberion_bringfocus_info_12_02': {'de_A': 'Yberion'},
      });

      expect(rows.whereType<DialogGroupRow>().map((group) => group.speaker), [
        'kalom',
        'yberion',
      ]);
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

  group('buildDialogRows with additionalIds', () {
    const catalog = <String, Map<String, String>>{
      'info_aaron_001': {'de_A': 'Hallo'},
      'info_bob_001': {'de_A': 'Moin'},
    };

    test('groups a new dialog id absent from an empty catalog', () {
      final rows = buildDialogRows(
        const {},
        additionalIds: const {'info_viper_gore_01'},
      );

      final group = rows.whereType<DialogGroupRow>().single;
      expect(group.speaker, 'viper');
      expect(group.lineCount, 1);
      expect(rows.whereType<DialogLineRow>().single.id, 'info_viper_gore_01');
    });

    test('onlyIds can select a new id while excluding catalog entries', () {
      final rows = buildDialogRows(
        catalog,
        onlyIds: const {'dia_newcomer_001'},
        additionalIds: const {'dia_newcomer_001'},
      );

      expect(rows.whereType<DialogLineRow>().map((line) => line.id), const [
        'dia_newcomer_001',
      ]);
      expect(rows.whereType<DialogGroupRow>().single.speaker, 'newcomer');
    });

    test('deduplicates an additional id already present in the catalog', () {
      final rows = buildDialogRows(
        catalog,
        additionalIds: const {'INFO_AARON_001'},
      );

      expect(
        rows.whereType<DialogLineRow>().where(
          (line) => line.id == 'info_aaron_001',
        ),
        hasLength(1),
      );
    });
  });

  group('isDialogLocId', () {
    test('accepts dialog/bark prefixes, rejects everything else', () {
      expect(isDialogLocId('info_aaron_001'), isTrue);
      expect(isDialogLocId('dia_alice_001'), isTrue);
      expect(isDialogLocId('gvl_zed_001'), isTrue);
      expect(isDialogLocId('svm_guard_001'), isTrue);
      expect(isDialogLocId('grd_263_asghan_open_info_06_02'), isTrue);
      expect(isDialogLocId('stt_302_viper_greet_info_11_02'), isTrue);
      expect(isDialogLocId('itfo_apple_name'), isFalse);
      expect(isDialogLocId('text_menu_001'), isFalse);
      expect(isDialogLocId('grd_armor_bot_h_01'), isFalse);
      // Whole-token match, not a substring prefix match.
      expect(isDialogLocId('information_x'), isFalse);
    });

    test('agrees with buildDialogRows inclusion per id', () {
      const catalog = <String, Map<String, String>>{
        'info_aaron_001': {'de_A': 'Hallo'},
        'svm_guard_001': {'de_A': 'Halt!'},
        'itfo_apple_name': {'de_A': 'Apfel'},
      };
      final included = buildDialogRows(
        catalog,
      ).whereType<DialogLineRow>().map((l) => l.id).toSet();
      for (final id in catalog.keys) {
        expect(isDialogLocId(id), included.contains(id), reason: id);
      }
    });
  });

  group('DialogRowsMemo', () {
    const catalog = <String, Map<String, String>>{
      'info_aaron_001': {'de_A': 'Hallo'},
      'info_bob_001': {'de_A': 'Moin'},
    };

    test('identical inputs return the identical rows list instance', () {
      final memo = DialogRowsMemo();
      final ids = {'info_aaron_001'};
      final a = memo.rowsFor(catalog, ids);
      final b = memo.rowsFor(catalog, ids);
      expect(identical(a, b), isTrue);
      expect(a.whereType<DialogLineRow>().single.id, 'info_aaron_001');
    });

    test('a new input identity rebuilds, even with equal content', () {
      final memo = DialogRowsMemo();
      final a = memo.rowsFor(catalog, {'info_aaron_001'});
      final b = memo.rowsFor(catalog, {'info_aaron_001'});
      expect(identical(a, b), isFalse);
      // The rebuilt result is still correct.
      expect(b.whereType<DialogLineRow>().single.id, 'info_aaron_001');
    });

    test('changing ids then reverting to a kept instance rebuilds once', () {
      final memo = DialogRowsMemo();
      final aaronOnly = {'info_aaron_001'};
      final both = {'info_aaron_001', 'info_bob_001'};
      final a = memo.rowsFor(catalog, aaronOnly);
      final b = memo.rowsFor(catalog, both);
      expect(b.whereType<DialogLineRow>(), hasLength(2));
      // Memo holds only the last input pair, so the old set rebuilds fresh.
      final c = memo.rowsFor(catalog, aaronOnly);
      expect(identical(a, c), isFalse);
      expect(c.whereType<DialogLineRow>().single.id, 'info_aaron_001');
    });

    test('changing additional ids invalidates the memo', () {
      final memo = DialogRowsMemo();
      const extra = {'info_viper_gore_01'};
      final a = memo.rowsFor(catalog, null, additionalIds: extra);
      final b = memo.rowsFor(catalog, null, additionalIds: extra);
      final c = memo.rowsFor(
        catalog,
        null,
        additionalIds: const {'info_viper_gore_02'},
      );

      expect(identical(a, b), isTrue);
      expect(identical(b, c), isFalse);
      expect(
        c.whereType<DialogLineRow>().map((line) => line.id),
        contains('info_viper_gore_02'),
      );
    });
  });
}
