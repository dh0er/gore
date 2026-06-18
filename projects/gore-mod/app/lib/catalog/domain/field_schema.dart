/// Type of a CDO field as declared in the UE4SS reflection model.
enum FieldType { int_, float_, bool_, string_, enum_ }

/// Schema for one editable CDO field on a UItemDefinition subclass.
/// Sourced from the parsed model.json; the GUI uses this to choose the
/// correct input widget and run client-side validation before calling
/// gore_core::validate_override.
class FieldSchema {
  const FieldSchema({
    required this.name,
    required this.type,
    this.minValue,
    this.maxValue,
    this.enumValues = const [],
  });

  final String name;
  final FieldType type;

  /// Inclusive lower bound — null means unconstrained.
  final num? minValue;

  /// Inclusive upper bound — null means unconstrained.
  final num? maxValue;

  /// Non-empty only when [type] == [FieldType.enum_].
  final List<String> enumValues;

  factory FieldSchema.fromJson(Map<String, Object?> json) {
    final rawType = json['type'] as String? ?? 'int';
    final type = switch (rawType) {
      'float' || 'double' => FieldType.float_,
      'bool'              => FieldType.bool_,
      'string'            => FieldType.string_,
      'enum'              => FieldType.enum_,
      _                   => FieldType.int_,
    };
    return FieldSchema(
      name:       json['name'] as String? ?? '',
      type:       type,
      minValue:   json['min'] as num?,
      maxValue:   json['max'] as num?,
      enumValues: (json['enum_values'] as List?)
                      ?.whereType<String>()
                      .toList() ??
                  const [],
    );
  }
}

/// The five scalar CDO fields proven editable on UItemDefinition.
/// Serves as the fallback field list when model.json doesn't enumerate
/// per-class fields (i.e., the model was built without --include-fields).
const List<FieldSchema> kDefaultItemFields = [
  FieldSchema(name: 'm_Value',    type: FieldType.int_,   minValue: 0),
  FieldSchema(name: 'm_MaxStack', type: FieldType.int_,   minValue: 1),
  FieldSchema(name: 'm_Weight',   type: FieldType.float_, minValue: 0),
  FieldSchema(name: 'm_Mass',     type: FieldType.float_, minValue: 0),
];
