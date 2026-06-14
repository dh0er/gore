import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';

void main() {
  test('parses catalog json', () {
    const json = '''
[
  {"category": "misc", "id": "ItMi_Orenugget", "path": "/Script/Angelscript.ItMi_Orenugget"},
  {"category": "rune", "id": "ItAr_Rune_FireBall", "path": "/Script/Angelscript.ItAr_Rune_FireBall"}
]''';
    final catalog = ItemCatalog.fromJsonString(json);
    expect(catalog.entries, hasLength(2));
    expect(catalog.entries.first.id, 'ItAr_Rune_FireBall'); // sorted by id
    expect(catalog.entries.first.category, 'rune');
  });

  testWidgets('loads bundled asset', (tester) async {
    // rootBundle does real async I/O; run outside the fake-async test zone.
    final catalog = (await tester.runAsync(ItemCatalog.loadBundled))!;
    expect(catalog.entries.length, greaterThan(500));
    expect(catalog.entries.any((e) => e.id == 'ItMi_Orenugget'), isTrue);
  });
}
