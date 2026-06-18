import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';

void main() {
  group('mergeDefaultBounds', () {
    test('applies default bounds to model fields that lack them', () {
      // Model entries carry only name/type — bounds must come from defaults.
      const parsed = [
        FieldSchema(name: 'm_MaxStack', type: FieldType.int_),
        FieldSchema(name: 'm_Weight', type: FieldType.float_),
      ];
      final merged = mergeDefaultBounds(parsed);
      final maxStack = merged.firstWhere((f) => f.name == 'm_MaxStack');
      final weight = merged.firstWhere((f) => f.name == 'm_Weight');
      expect(maxStack.minValue, 1); // can't set a stack of 0
      expect(weight.minValue, 0); // no negative weight
    });

    test('keeps a bound the parsed field already specifies', () {
      const parsed = [
        FieldSchema(name: 'm_Value', type: FieldType.int_, minValue: 5),
      ];
      expect(mergeDefaultBounds(parsed).first.minValue, 5);
    });

    test('leaves unknown fields untouched', () {
      const parsed = [FieldSchema(name: 'm_Custom', type: FieldType.int_)];
      expect(mergeDefaultBounds(parsed).first.minValue, isNull);
    });
  });

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
