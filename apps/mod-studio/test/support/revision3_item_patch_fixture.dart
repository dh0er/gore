Map<String, Object?> revision3ItemNumericField({
  required String name,
  required String scalarType,
  Object? defaultValue,
}) {
  final (domain, minimum, maximum) = switch (scalarType) {
    'integer' => (
      'signed_integer32',
      <String, Object?>{'type': 'integer', 'data': -0x80000000},
      <String, Object?>{'type': 'integer', 'data': 0x7fffffff},
    ),
    'float' => (
      'finite_float32',
      <String, Object?>{'type': 'float', 'data': -3.4028234663852886e38},
      <String, Object?>{'type': 'float', 'data': 3.4028234663852886e38},
    ),
    _ => throw ArgumentError.value(
      scalarType,
      'scalarType',
      'expected integer or float',
    ),
  };
  final field = <String, Object?>{
    'name': name,
    'scalar_type': scalarType,
    'numeric_domain': domain,
    'minimum_value': minimum,
    'maximum_value': maximum,
  };
  if (defaultValue != null) field['default_value'] = defaultValue;
  return field;
}
