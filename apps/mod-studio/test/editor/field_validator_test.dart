import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';
import 'package:gore_mod/editor/domain/field_validator.dart';

void main() {
  const intSchema   = FieldSchema(name: 'm_Value',  type: FieldType.int_,   minValue: 0, maxValue: 1000000);
  const floatSchema = FieldSchema(name: 'm_Weight', type: FieldType.float_, minValue: 0);
  const boolSchema  = FieldSchema(name: 'm_Flag',   type: FieldType.bool_);
  const enumSchema  = FieldSchema(name: 'm_Quality', type: FieldType.enum_, enumValues: ['Low', 'Medium', 'High']);

  group('int field', () {
    test('accepts valid integer', ()    => expect(validateField(intSchema, '500'), isNull));
    test('rejects non-integer',  ()    => expect(validateField(intSchema, 'abc'), isNotNull));
    test('rejects float string', ()    => expect(validateField(intSchema, '1.5'), isNotNull));
    test('rejects below minimum', ()   => expect(validateField(intSchema, '-1'), isNotNull));
    test('rejects above maximum', ()   => expect(validateField(intSchema, '2000000'), isNotNull));
    test('rejects empty string', ()    => expect(validateField(intSchema, ''), isNotNull));
  });

  group('float field', () {
    test('accepts valid float', ()     => expect(validateField(floatSchema, '1.5'), isNull));
    test('accepts whole number', ()    => expect(validateField(floatSchema, '2'), isNull));
    test('rejects non-numeric', ()     => expect(validateField(floatSchema, 'abc'), isNotNull));
    test('rejects negative min', ()    => expect(validateField(floatSchema, '-0.1'), isNotNull));
    test('rejects NaN', ()             => expect(validateField(floatSchema, 'NaN'), isNotNull));
    test('rejects Infinity', ()        => expect(validateField(floatSchema, 'Infinity'), isNotNull));
    test('rejects overflow literal', () => expect(validateField(floatSchema, '1e309'), isNotNull));
  });

  group('bool field', () {
    test('accepts true', ()  => expect(validateField(boolSchema, 'true'), isNull));
    test('accepts false', () => expect(validateField(boolSchema, 'false'), isNull));
    test('rejects 1', ()     => expect(validateField(boolSchema, '1'), isNotNull));
  });

  group('enum field', () {
    test('accepts known value', ()   => expect(validateField(enumSchema, 'Medium'), isNull));
    test('rejects unknown value', () => expect(validateField(enumSchema, 'Ultra'), isNotNull));
  });

  group('parsedValue', () {
    test('int', ()    => expect(parsedValue(intSchema,   '500'), 500));
    test('float', ()  => expect(parsedValue(floatSchema, '1.5'), 1.5));
    test('bool', ()   => expect(parsedValue(boolSchema,  'true'), true));
    // Enum resolves to the member's backing int — the index as a fallback when
    // no explicit backing values are known.
    test('enum -> index when no backing values', () {
      expect(parsedValue(enumSchema, 'Low'), 0);
      expect(parsedValue(enumSchema, 'High'), 2);
    });

    test('enum -> declared backing value (non-contiguous)', () {
      const s = FieldSchema(
        name: 'm_Q',
        type: FieldType.enum_,
        enumValues: ['Low', 'Mid', 'High'],
        enumBackingValues: [0, 5, 9],
      );
      expect(parsedValue(s, 'Mid'), 5);
      expect(parsedValue(s, 'High'), 9);
    });
  });
}
