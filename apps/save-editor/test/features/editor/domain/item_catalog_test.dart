import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';

void main() {
  test('parses catalog json', () {
    const json = '''
[
  {"category": "misc", "icon": "ItMi_Orenugget", "id": "ItMi_Orenugget", "path": "/Script/Angelscript.ItMi_Orenugget"},
  {"category": "rune", "icon": "ItAr_Rune_FireBall", "id": "ItAr_Rune_FireBall", "path": "/Script/Angelscript.ItAr_Rune_FireBall"}
]''';
    final catalog = ItemCatalog.fromJsonString(json);
    expect(catalog.entries, hasLength(2));
    expect(catalog.entries.first.id, 'ItAr_Rune_FireBall'); // sorted by id
    expect(catalog.entries.first.category, 'rune');
    expect(catalog.entries.first.icon, 'ItAr_Rune_FireBall');
  });

  testWidgets('loads bundled asset', (tester) async {
    // rootBundle does real async I/O; run outside the fake-async test zone.
    final catalog = (await tester.runAsync(ItemCatalog.loadBundled))!;
    expect(catalog.entries, hasLength(831));
    expect(catalog.entries.map((e) => e.id).toSet(), hasLength(831));
    expect(catalog.entries.map((e) => e.path).toSet(), hasLength(831));
    expect(
      catalog.entries.map((e) => e.id.toLowerCase()).toSet(),
      hasLength(831),
    );
    expect(
      catalog.entries.map((e) => e.path.toLowerCase()).toSet(),
      hasLength(831),
    );
    for (final entry in catalog.entries) {
      expect(entry.path, '/Script/Angelscript.${entry.id}');
      expect(entry.icon, matches(RegExp(r'^[A-Za-z0-9_]+$')));
    }
    final byId = {for (final entry in catalog.entries) entry.id: entry};
    expect(byId['ItMi_Orenugget']?.icon, 'ItMi_Orenugget');
    expect(byId['ItMi_Stuff_Brush']?.icon, 'ItMi_Stuff_Brush');
    expect(byId['ItMi_Oldcoin_01']?.icon, 'ItMi_Oldcoin_01');
    expect(
      catalog.entries.where((entry) => entry.icon == entry.id),
      hasLength(676),
    );
    expect(
      catalog.entries,
      everyElement(predicate<ItemCatalogEntry>((e) => e.icon.isNotEmpty)),
    );
  });
}
