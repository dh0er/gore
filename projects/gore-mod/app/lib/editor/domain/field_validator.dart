import '../../catalog/domain/field_schema.dart';

/// Validates [raw] (user-entered string) against [schema].
/// Returns null when valid, an error message when invalid.
String? validateField(FieldSchema schema, String raw) {
  final trimmed = raw.trim();
  if (trimmed.isEmpty) return 'Required';

  switch (schema.type) {
    case FieldType.int_:
      final n = int.tryParse(trimmed);
      if (n == null) return 'Must be a whole number';
      if (schema.minValue != null && n < schema.minValue!) {
        return 'Must be ≥ ${schema.minValue}';
      }
      if (schema.maxValue != null && n > schema.maxValue!) {
        return 'Must be ≤ ${schema.maxValue}';
      }
      return null;

    case FieldType.float_:
      final f = double.tryParse(trimmed);
      if (f == null) return 'Must be a number';
      // double.tryParse accepts NaN / Infinity / overflowing literals like
      // 1e309. Those slip past the min/max checks below (NaN compares false to
      // everything) and then crash jsonEncode when the override is sent to the
      // native FFI, so reject them up front.
      if (!f.isFinite) return 'Must be a finite number';
      if (schema.minValue != null && f < schema.minValue!) {
        return 'Must be ≥ ${schema.minValue}';
      }
      if (schema.maxValue != null && f > schema.maxValue!) {
        return 'Must be ≤ ${schema.maxValue}';
      }
      return null;

    case FieldType.bool_:
      if (trimmed != 'true' && trimmed != 'false') {
        return 'Must be true or false';
      }
      return null;

    case FieldType.string_:
      return null; // No further constraint at the GUI layer.

    case FieldType.enum_:
      if (!schema.enumValues.contains(trimmed)) {
        return 'Must be one of: ${schema.enumValues.join(', ')}';
      }
      return null;
  }
}

/// Converts a validated raw string to the JSON-serialisable value that
/// gore_core's `generate_mod` expects.
///
/// Enum fields resolve to their backing integer (the member's index in
/// declaration order), NOT the member name: gore_core treats UE enum CDO
/// fields as int-backed and only accepts `value_int` for them, so emitting the
/// name as `value_str` would generate Lua that assigns a string to an int
/// field. `validateField` has already confirmed membership, so `indexOf` is
/// always ≥ 0 here.
Object parsedValue(FieldSchema schema, String raw) {
  final trimmed = raw.trim();
  return switch (schema.type) {
    FieldType.int_    => int.parse(trimmed),
    FieldType.float_  => double.parse(trimmed),
    FieldType.bool_   => trimmed == 'true',
    FieldType.string_ => trimmed,
    FieldType.enum_   => _enumBackingValue(schema, trimmed),
  };
}

/// The backing integer for an enum member name: its declared discriminant when
/// known ([FieldSchema.enumBackingValues]), else the member index as a fallback
/// (contiguous 0-based enums). `validateField` has already confirmed membership.
int _enumBackingValue(FieldSchema schema, String member) {
  final i = schema.enumValues.indexOf(member);
  if (i < 0) return 0;
  if (i < schema.enumBackingValues.length) return schema.enumBackingValues[i];
  return i;
}
