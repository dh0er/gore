import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';

void main() {
  group('FieldSchema.fromJson', () {
    test('parses type, enum_values and default', () {
      final f = FieldSchema.fromJson({
        'name': 'm_Value',
        'type': 'int',
        'default': 4,
      });
      expect(f.name, 'm_Value');
      expect(f.type, FieldType.int_);
      expect(f.defaultValue, 4);
    });

    test('default is null when absent (header-derived model)', () {
      final f = FieldSchema.fromJson({'name': 'm_X', 'type': 'float'});
      expect(f.defaultValue, isNull);
    });

    test('enum default is the backing index alongside enum_values', () {
      final f = FieldSchema.fromJson({
        'name': 'm_Quality',
        'type': 'enum',
        'enum_values': ['Low', 'Medium', 'High'],
        'default': 2,
      });
      expect(f.type, FieldType.enum_);
      expect(f.enumValues, ['Low', 'Medium', 'High']);
      expect(f.defaultValue, 2);
    });
  });
}
