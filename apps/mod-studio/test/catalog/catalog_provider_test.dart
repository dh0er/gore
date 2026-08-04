import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';

void main() {
  group('editableFields', () {
    test('drops enum fields with no choices', () {
      const parsed = [
        FieldSchema(name: 'm_Value', type: FieldType.int_),
        FieldSchema(name: 'm_Quality', type: FieldType.enum_), // no enumValues
      ];
      final usable = editableFields(parsed);
      expect(usable.map((f) => f.name), ['m_Value']);
    });

    test('keeps enum fields that have choices', () {
      const parsed = [
        FieldSchema(
          name: 'm_Quality',
          type: FieldType.enum_,
          enumValues: ['Low', 'High'],
        ),
      ];
      expect(editableFields(parsed), hasLength(1));
    });
  });

  group('_fieldsFor (via CatalogItem.fromCatalogEntry)', () {
    test('keeps an item non-editable when class evidence is absent', () {
      final item = CatalogItem.fromCatalogEntry({
        'id': 'ItFo_Apple',
      }, fields: const <FieldSchema>[]);
      expect(item.fields, isEmpty);
    });

    test('uses parsed fields when present', () {
      const fields = [FieldSchema(name: 'm_Value', type: FieldType.int_)];
      final item = CatalogItem.fromCatalogEntry({
        'id': 'ItFo_Apple',
      }, fields: fields);
      expect(item.fields.first.name, 'm_Value');
    });

    test('does not add guessed fields when fields list is empty', () {
      final item = CatalogItem.fromCatalogEntry({
        'id': 'ItFo_Apple',
      }, fields: const <FieldSchema>[]);
      expect(item.fields, isEmpty);
    });
  });
}
