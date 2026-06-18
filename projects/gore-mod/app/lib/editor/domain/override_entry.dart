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

  /// Serialize as one `SingleOverride` for the gore_core `generate_mod` FFI:
  /// the value goes under a typed key (`value_int` / `value_float` /
  /// `value_bool` / `value_str`) that matches the Rust `OverrideValue` flatten
  /// representation. Sending a generic `value` key makes gore_core reject the
  /// config with `BAD_CONFIG`.
  Map<String, Object?> toFfiJson() {
    final v = newValue;
    final String valueKey;
    if (v is bool) {
      valueKey = 'value_bool';
    } else if (v is int) {
      valueKey = 'value_int';
    } else if (v is double) {
      valueKey = 'value_float';
    } else {
      valueKey = 'value_str';
    }
    return {
      'class': classId,
      'field': field,
      valueKey: v,
    };
  }

  OverrideEntry copyWith({Object? newValue}) =>
      OverrideEntry(
        classId:  classId,
        field:    field,
        oldValue: oldValue,
        newValue: newValue ?? this.newValue,
      );
}
