/// One pending CDO override — mirrors one `[[override]]` block in overrides.toml.
class OverrideEntry {
  const OverrideEntry({
    required this.classId,
    required this.field,
    required this.oldValue,
    required this.newValue,
  });

  /// Angelscript short class name (e.g. `ItFo_Apple`).
  final String classId;

  /// CDO field name (e.g. `m_Value`).
  final String field;

  /// Default value from the catalog/model (display only).
  final Object? oldValue;

  /// New value to apply (int | double | bool | String).
  final Object newValue;

  /// Unique key for the override map: `classId.field`.
  String get key => '$classId.$field';

  Map<String, Object?> toJson() => {
    'class': classId,
    'field': field,
    'value': newValue,
  };

  OverrideEntry copyWith({Object? newValue}) =>
      OverrideEntry(
        classId:  classId,
        field:    field,
        oldValue: oldValue,
        newValue: newValue ?? this.newValue,
      );
}
