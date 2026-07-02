import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart'
    show sortByDisplayName;

CatalogItem _item(String id) =>
    CatalogItem(id: id, displayName: id, fields: const []);

void main() {
  test('sorts by localized name, case-insensitive, id tiebreak', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
      _item('ItMw_Beil'),
    ];
    final names = {
      'ItMw_Zweihander': 'Anderthalbhänder',
      'ItMw_Axt': 'zerbrochene Axt',
      'ItMw_Beil': 'Beil',
    };
    final sorted = sortByDisplayName(items, (i) => names[i.id]!);
    expect(sorted.map((i) => names[i.id]).toList(),
        ['Anderthalbhänder', 'Beil', 'zerbrochene Axt']);
  });

  test('falls back to id order when names are equal', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
    ];
    // Both map to the same display name -> id decides.
    final sorted = sortByDisplayName(items, (_) => 'Schwert');
    expect(sorted.map((i) => i.id).toList(),
        ['ItMw_Axt', 'ItMw_Zweihander']);
  });

  test('does not mutate the input list', () {
    final items = [
      _item('ItMw_Zweihander'),
      _item('ItMw_Axt'),
    ];
    sortByDisplayName(items, (i) => i.id);
    expect(items.map((i) => i.id).toList(),
        ['ItMw_Zweihander', 'ItMw_Axt']);
  });
}
