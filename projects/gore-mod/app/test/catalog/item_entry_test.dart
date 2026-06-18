import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';

void main() {
  group('CatalogItem.fromCatalogEntry', () {
    test('derives display name by stripping prefix', () {
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'});
      expect(item.id, 'ItFo_Apple');
      expect(item.displayName, 'Apple');
    });

    test('keeps id when no known prefix matches', () {
      final item = CatalogItem.fromCatalogEntry({'id': 'UnknownThing'});
      expect(item.displayName, 'UnknownThing');
    });

    test('defaults to kDefaultItemFields when no fields supplied', () {
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'});
      expect(item.fields, kDefaultItemFields);
    });

    test('uses supplied fields when provided', () {
      const custom = [FieldSchema(name: 'm_Value', type: FieldType.int_)];
      final item = CatalogItem.fromCatalogEntry({'id': 'ItFo_Apple'}, fields: custom);
      expect(item.fields, custom);
    });
  });

  group('FieldSchema.fromJson', () {
    test('parses int field', () {
      final s = FieldSchema.fromJson({'name': 'm_Value', 'type': 'int', 'min': 0});
      expect(s.type, FieldType.int_);
      expect(s.minValue, 0);
    });

    test('parses float field', () {
      final s = FieldSchema.fromJson({'name': 'm_Weight', 'type': 'float'});
      expect(s.type, FieldType.float_);
    });

    test('parses enum field with values', () {
      final s = FieldSchema.fromJson({
        'name': 'm_Quality',
        'type': 'enum',
        'enum_values': ['Low', 'Medium', 'High'],
      });
      expect(s.type, FieldType.enum_);
      expect(s.enumValues, ['Low', 'Medium', 'High']);
    });
  });
}
