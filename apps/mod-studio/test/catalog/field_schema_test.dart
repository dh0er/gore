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
        FieldSchema.fromJson({
          'name': 'a',
          'type': 'int',
          'default': '4.0',
        }).defaultValue,
        isNull,
      );
      // int field with a double default -> dropped (would crash int.parse).
      expect(
        FieldSchema.fromJson({
          'name': 'b',
          'type': 'int',
          'default': 4.0,
        }).defaultValue,
        isNull,
      );
      // float accepts an int default.
      expect(
        FieldSchema.fromJson({
          'name': 'c',
          'type': 'float',
          'default': 4,
        }).defaultValue,
        4,
      );
      // bool with a string default -> dropped.
      expect(
        FieldSchema.fromJson({
          'name': 'd',
          'type': 'bool',
          'default': 'true',
        }).defaultValue,
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

    test('plain parser does not infer a native numeric domain', () {
      final f = FieldSchema.fromJson({'name': 'm_Value', 'type': 'int'});
      expect(f.numericDomain, isNull);
    });
  });

  group('FieldSchema.fromItemModelJson', () {
    test('attaches only verified int32 and float32 item domains', () {
      expect(
        FieldSchema.fromItemModelJson({
          'name': 'm_Value',
          'type': 'int',
        }).numericDomain,
        FieldNumericDomain.signedInteger32,
      );
      expect(
        FieldSchema.fromItemModelJson({
          'name': 'm_Weight',
          'type': 'float',
        }).numericDomain,
        FieldNumericDomain.finiteFloat32,
      );
      expect(
        FieldSchema.fromItemModelJson({
          'name': 'm_AutoTarget',
          'type': 'bool',
        }).type,
        FieldType.bool_,
      );
    });

    test('rejects matching names with different or missing raw types', () {
      expect(
        () => FieldSchema.fromItemModelJson({
          'name': 'm_Weight',
          'type': 'double',
        }),
        throwsFormatException,
      );
      expect(
        () =>
            FieldSchema.fromItemModelJson({'name': 'm_Value', 'type': 'int64'}),
        throwsFormatException,
      );
      expect(
        () => FieldSchema.fromItemModelJson({'name': 'm_Value'}),
        throwsFormatException,
      );
      expect(
        () => FieldSchema.fromItemModelJson({
          'name': 'm_Unverified',
          'type': 'int',
        }),
        throwsFormatException,
      );
    });

    test('drops a verified field default outside its native domain', () {
      expect(
        FieldSchema.fromItemModelJson({
          'name': 'm_Value',
          'type': 'int',
          'default': 0x80000000,
        }).defaultValue,
        isNull,
      );
      expect(
        FieldSchema.fromItemModelJson({
          'name': 'm_Weight',
          'type': 'float',
          'default': 1e39,
        }).defaultValue,
        isNull,
      );
    });
  });
}
