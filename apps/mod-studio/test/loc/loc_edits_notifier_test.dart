import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/loc/domain/loc_edits_notifier.dart';

void main() {
  ProviderContainer makeContainer() => ProviderContainer();

  test('setEdit stages an edit; entryCount/isDirty reflect it', () {
    final c = makeContainer();
    addTearDown(c.dispose);
    final n = c.read(locEditsProvider.notifier);

    expect(c.read(locEditsProvider).isDirty, false);
    n.setEdit('itfo_cheese', 'german_new', 'Käse-Mod');
    final s = c.read(locEditsProvider);
    expect(s.isDirty, true);
    expect(s.entryCount, 1);
    expect(s.editFor('itfo_cheese', 'german_new'), 'Käse-Mod');
  });

  test('removeEdit removes exactly one language set; others stay', () {
    final c = makeContainer();
    addTearDown(c.dispose);
    final n = c.read(locEditsProvider.notifier);
    n.setEdit('dia_x', 'german_new', 'Hallo');
    n.setEdit('dia_x', 'french', 'Salut');
    n.setEdit('dia_y', 'german_new', 'Moin');
    n.removeEdit('dia_x', 'german_new');
    final s = c.read(locEditsProvider);
    expect(s.editFor('dia_x', 'german_new'), isNull);
    expect(s.editFor('dia_x', 'french'), 'Salut');
    expect(s.editFor('dia_y', 'german_new'), 'Moin');
    expect(s.entryCount, 2);
  });

  test('removeEdit drops the set; clearing last set drops the id', () {
    final c = makeContainer();
    addTearDown(c.dispose);
    final n = c.read(locEditsProvider.notifier);
    n.setEdit('dia_x', 'english_newer', 'Hi');
    n.removeEdit('dia_x', 'english_newer');
    expect(c.read(locEditsProvider).edits.containsKey('dia_x'), false);
    expect(c.read(locEditsProvider).isDirty, false);
  });

  test('clearForId and clearAll', () {
    final c = makeContainer();
    addTearDown(c.dispose);
    final n = c.read(locEditsProvider.notifier);
    n.setEdit('a', 's', '1');
    n.setEdit('b', 's', '2');
    n.clearForId('a');
    expect(c.read(locEditsProvider).edits.keys, ['b']);
    n.clearAll();
    expect(c.read(locEditsProvider).isDirty, false);
  });

  test('loadAll replaces the whole edit set', () {
    final c = makeContainer();
    addTearDown(c.dispose);
    final n = c.read(locEditsProvider.notifier);
    n.setEdit('old', 's', 'x');
    n.loadAll({
      'p': {'english_newer': 'A'},
      'q': {'german_new': 'B'},
    });
    final s = c.read(locEditsProvider);
    expect(s.edits.containsKey('old'), false);
    expect(s.entryCount, 2);
    expect(s.editFor('q', 'german_new'), 'B');
  });
}
