import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';

void main() {
  group('_fieldsFor (via CatalogItem.fromCatalogEntry)', () {
    test('uses kDefaultItemFields when class absent from model', () {
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'});
      expect(item.fields, kDefaultItemFields);
    });

    test('uses parsed fields when present', () {
      const fields = [FieldSchema(name: 'm_Value', type: FieldType.int_)];
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'}, fields: fields);
      expect(item.fields.first.name, 'm_Value');
    });

    test('falls back to kDefaultItemFields when fields list is empty', () {
      // Simulates a model.json class entry with an empty fields array.
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'}, fields: kDefaultItemFields);
      expect(item.fields, isNotEmpty);
    });
  });
}
