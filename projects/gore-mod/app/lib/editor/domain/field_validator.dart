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
/// gore_core's validate_override / generate_mod expect.
Object parsedValue(FieldSchema schema, String raw) {
  final trimmed = raw.trim();
  return switch (schema.type) {
    FieldType.int_    => int.parse(trimmed),
    FieldType.float_  => double.parse(trimmed),
    FieldType.bool_   => trimmed == 'true',
    FieldType.string_ => trimmed,
    FieldType.enum_   => trimmed,
  };
}
