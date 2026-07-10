import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart'
    show sortByLocalizedName;

CatalogItem _item(String id) =>
    CatalogItem(id: id, displayName: id, fields: const []);

void main() {
  test('sorts by localized name, case-insensitive, id tiebreak', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
      _item('ItMw_Beil'),
    ];
    const names = {
      'ItMw_Zweihander': 'Anderthalbhänder',
      'ItMw_Axt': 'zerbrochene Axt',
      'ItMw_Beil': 'Beil',
    };
    final sorted = sortByLocalizedName(items, (i) => names[i.id]!);
    expect(sorted.map((i) => names[i.id]).toList(),
        ['Anderthalbhänder', 'Beil', 'zerbrochene Axt']);
  });

  test('is case-insensitive (lowercase does not sort after uppercase)', () {
    // Case-sensitive code-unit order would put 'Beil' before 'apfel'.
    final items = [_item('ItMw_Beil'), _item('ItFo_Apple')];
    const names = {
      'ItFo_Apple': 'apfel',
      'ItMw_Beil': 'Beil',
    };
    final sorted = sortByLocalizedName(items, (i) => names[i.id]!);
    expect(sorted.map((i) => names[i.id]).toList(), ['apfel', 'Beil']);
  });

  test('folds umlauts so Ö sorts near O, not after Z', () {
    // Code-unit order (even lowercased) would put 'Öllampe' after 'Zweihänder'.
    final items = [_item('ItMw_Zweihander'), _item('ItMi_Lampe')];
    const names = {
      'ItMi_Lampe': 'Öllampe',
      'ItMw_Zweihander': 'Zweihänder',
    };
    final sorted = sortByLocalizedName(items, (i) => names[i.id]!);
    expect(sorted.map((i) => names[i.id]).toList(), ['Öllampe', 'Zweihänder']);
  });

  test('falls back to id order when names are equal', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
    ];
    // Both map to the same display name -> id decides.
    final sorted = sortByLocalizedName(items, (_) => 'Schwert');
    expect(sorted.map((i) => i.id).toList(),
        ['ItMw_Axt', 'ItMw_Zweihander']);
  });

  test('does not mutate the input list', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
    ];
    sortByLocalizedName(items, (i) => i.id);
    expect(items.map((i) => i.id).toList(),
        ['ItMw_Zweihander', 'ItMw_Axt']);
  });
}
