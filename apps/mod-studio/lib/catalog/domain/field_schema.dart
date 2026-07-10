/// Type of a CDO field as declared in the UE4SS reflection model.
enum FieldType { int_, float_, bool_, string_, enum_ }

/// Schema for one editable CDO field on a UItemDefinition subclass.
/// Sourced from the parsed model.json; the GUI uses this to choose the
/// correct input widget and run client-side validation before calling
/// gore_modgen::validate_config.
class FieldSchema {
  const FieldSchema({
    required this.name,
    required this.type,
    this.minValue,
    this.maxValue,
    this.enumValues = const [],
    this.enumBackingValues = const [],
    this.defaultValue,
  });

  final String name;
  final FieldType type;

  /// Inclusive lower bound — null means unconstrained.
  final num? minValue;

  /// Inclusive upper bound — null means unconstrained.
  final num? maxValue;

  /// Non-empty only when [type] == [FieldType.enum_].
  final List<String> enumValues;

  /// Backing integer per enum member (parallel to [enumValues]). The override
  /// stores this value, not the member index, so non-contiguous discriminants
  /// (e.g. `Mid = 5`) round-trip. Empty when unknown — callers then fall back
  /// to the index.
  final List<int> enumBackingValues;

  /// The field's real CDO default value, when the model came from a runtime
  /// dump (`gore-cli sync`). null when unknown (header-derived model) — the
  /// editor then shows a placeholder. For enum fields this is the backing
  /// integer (member index), matching how overrides are encoded.
  final Object? defaultValue;

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
      enumBackingValues: (json['enum_value_ints'] as List?)
                      ?.whereType<num>()
                      .map((n) => n.toInt())
                      .toList() ??
                  const [],
      // A user-loaded dump is read directly (no `gore-cli sync` validation), so
      // drop a default whose JSON type doesn't match the field — otherwise
      // _defaultText -> parsedValue would throw on the first edit.
      defaultValue: _coerceDefault(type, json['default']),
    );
  }

  static Object? _coerceDefault(FieldType type, Object? d) {
    if (d == null) return null;
    return switch (type) {
      // Enum defaults are the backing integer.
      FieldType.int_ || FieldType.enum_ => d is int ? d : null,
      FieldType.float_  => d is num ? d : null,
      FieldType.bool_   => d is bool ? d : null,
      FieldType.string_ => d is String ? d : null,
    };
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
