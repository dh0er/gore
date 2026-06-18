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

    test('drops a default whose type mismatches the field', () {
      // int field with a string default (unvalidated user dump) -> dropped.
      expect(
        FieldSchema.fromJson({'name': 'a', 'type': 'int', 'default': '4.0'}).defaultValue,
        isNull,
      );
      // int field with a double default -> dropped (would crash int.parse).
      expect(
        FieldSchema.fromJson({'name': 'b', 'type': 'int', 'default': 4.0}).defaultValue,
        isNull,
      );
      // float accepts an int default.
      expect(
        FieldSchema.fromJson({'name': 'c', 'type': 'float', 'default': 4}).defaultValue,
        4,
      );
      // bool with a string default -> dropped.
      expect(
        FieldSchema.fromJson({'name': 'd', 'type': 'bool', 'default': 'true'}).defaultValue,
        isNull,
      );
    });

    test('enum parses members, backing values and default', () {
      final f = FieldSchema.fromJson({
        'name': 'm_Quality',
        'type': 'enum',
        'enum_values': ['Low', 'Mid', 'High'],
        'enum_value_ints': [0, 5, 9],
        'default': 5,
      });
      expect(f.type, FieldType.enum_);
      expect(f.enumValues, ['Low', 'Mid', 'High']);
      expect(f.enumBackingValues, [0, 5, 9]);
      expect(f.defaultValue, 5);
    });
  });
}
