/// Type of a CDO field as declared in the UE4SS reflection model.
enum FieldType { int_, float_, bool_, string_, enum_ }

/// Storage domain proven for an editable numeric item field.
///
/// This is deliberately separate from gameplay bounds such as a minimum
/// weight or stack size. A domain describes only the native scalar type.
enum FieldNumericDomain { signedInteger32, finiteFloat32 }

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
    this.numericDomain,
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

  /// Native scalar domain when independently proven. null means the model only
  /// establishes the broad GUI type and no narrower storage domain is known.
  final FieldNumericDomain? numericDomain;

  factory FieldSchema.fromJson(Map<String, Object?> json) =>
      FieldSchema._fromJson(json);

  /// Parses one field from the bundled item model and attaches only storage
  /// domains verified against the shipped AngelScript `Binds.Cache`.
  ///
  /// Matching includes the raw model type: a future `double` or differently
  /// typed field with the same name must not inherit a float32/int32 claim.
  factory FieldSchema.fromItemModelJson(Map<String, Object?> json) {
    if (!isProvenItemModelField(json)) {
      throw const FormatException(
        'item model field has no exact native type evidence',
      );
    }
    final rawType = json['type']! as String;
    final numericDomain = switch (rawType) {
      'int' => FieldNumericDomain.signedInteger32,
      'float' => FieldNumericDomain.finiteFloat32,
      _ => null,
    };
    return FieldSchema._fromJson(json, numericDomain: numericDomain);
  }

  static FieldSchema _fromJson(
    Map<String, Object?> json, {
    FieldNumericDomain? numericDomain,
  }) {
    final rawType = json['type'] as String? ?? 'int';
    final type = switch (rawType) {
      'float' || 'double' => FieldType.float_,
      'bool' => FieldType.bool_,
      'string' => FieldType.string_,
      'enum' => FieldType.enum_,
      _ => FieldType.int_,
    };
    return FieldSchema(
      name: json['name'] as String? ?? '',
      type: type,
      minValue: json['min'] as num?,
      maxValue: json['max'] as num?,
      enumValues:
          (json['enum_values'] as List?)?.whereType<String>().toList() ??
          const [],
      enumBackingValues:
          (json['enum_value_ints'] as List?)
              ?.whereType<num>()
              .map((n) => n.toInt())
              .toList() ??
          const [],
      // A user-loaded dump is read directly (no `gore-cli sync` validation), so
      // drop a default whose JSON type doesn't match the field — otherwise
      // _defaultText -> parsedValue would throw on the first edit.
      defaultValue: _coerceDefault(type, json['default'], numericDomain),
      numericDomain: numericDomain,
    );
  }

  static Object? _coerceDefault(
    FieldType type,
    Object? d,
    FieldNumericDomain? numericDomain,
  ) {
    if (d == null) return null;
    final typed = switch (type) {
      // Enum defaults are the backing integer.
      FieldType.int_ || FieldType.enum_ => d is int ? d : null,
      FieldType.float_ => d is num ? d : null,
      FieldType.bool_ => d is bool ? d : null,
      FieldType.string_ => d is String ? d : null,
    };
    return switch ((numericDomain, typed)) {
      (FieldNumericDomain.signedInteger32, final int value)
          when value >= -0x80000000 && value <= 0x7fffffff =>
        value,
      (FieldNumericDomain.finiteFloat32, final num value)
          when value.isFinite &&
              value >= -3.4028234663852886e38 &&
              value <= 3.4028234663852886e38 =>
        value,
      (null, _) => typed,
      _ => null,
    };
  }
}

/// Whether a model field is one of the exact `(name, raw type)` pairs proven
/// against the sealed shipped AngelScript Binds cache.
///
/// A user-selected dump is untrusted input. Unknown names and plausible but
/// different types are hidden instead of being exposed through the writable
/// classic Item editor.
bool isProvenItemModelField(Map<String, Object?> json) {
  final name = json['name'];
  final rawType = json['type'];
  if (name is! String || rawType is! String) return false;
  return switch (rawType) {
    'int' => _signedInteger32ItemFieldNames.contains(name),
    'float' => _finiteFloat32ItemFieldNames.contains(name),
    'bool' => _booleanItemFieldNames.contains(name),
    _ => false,
  };
}

// Exact item-field names whose raw `int` declarations were verified as
// AngelScript `int` (signed i32) in the shipped Binds.Cache.
const Set<String> _signedInteger32ItemFieldNames = {
  'RequiredMagicCircleLevel',
  'm_MaxStack',
  'm_Value',
};

// Exact item-field names whose raw `float` declarations were verified as
// AngelScript `float32` in the shipped Binds.Cache.
const Set<String> _finiteFloat32ItemFieldNames = {
  'm_ArcParam',
  'm_ArrowGravityModifier',
  'm_BlockSuperArmorMultiplier',
  'm_Buoyancy',
  'm_CriticalMultiplier',
  'm_DamageReduction',
  'm_HpRegenerateTick',
  'm_Mass',
  'm_MaxRange',
  'm_Radius',
  'm_StartRegenerateSc',
  'm_SuperArmorDamageBase',
  'm_Weight',
};

// Exact item-field names whose raw `bool` declarations were verified in the
// shipped Binds.Cache. Bool needs no numeric domain but still requires the same
// closed name/type evidence before the classic writer may expose it.
const Set<String> _booleanItemFieldNames = {
  'm_AutoTarget',
  'm_CanEquipAfterUse',
  'm_IsTargetingIndicatorEnabled',
};
