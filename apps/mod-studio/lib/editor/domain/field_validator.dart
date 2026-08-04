import '../../catalog/domain/field_schema.dart';

/// A field validation failure, decoupled from any language. The UI maps each
/// case to a localized message via `AppLocalizations` (see `fieldErrorText`).
sealed class FieldError {
  const FieldError();
}

class RequiredFieldError extends FieldError {
  const RequiredFieldError();
}

class NotWholeNumberError extends FieldError {
  const NotWholeNumberError();
}

class NotANumberError extends FieldError {
  const NotANumberError();
}

class NotFiniteError extends FieldError {
  const NotFiniteError();
}

class BelowMinimumError extends FieldError {
  const BelowMinimumError(this.minValue);
  final num minValue;
}

class AboveMaximumError extends FieldError {
  const AboveMaximumError(this.maxValue);
  final num maxValue;
}

class NotBoolError extends FieldError {
  const NotBoolError();
}

class NotInEnumError extends FieldError {
  const NotInEnumError(this.allowed);
  final List<String> allowed;
}

/// Validates [raw] (user-entered string) against [schema].
/// Returns null when valid, a [FieldError] describing the problem otherwise.
FieldError? validateField(FieldSchema schema, String raw) {
  final trimmed = raw.trim();
  if (trimmed.isEmpty) return const RequiredFieldError();

  switch (schema.type) {
    case FieldType.int_:
      final n = int.tryParse(trimmed);
      if (n == null) return const NotWholeNumberError();
      if (schema.numericDomain == FieldNumericDomain.signedInteger32) {
        if (n < -0x80000000) return const BelowMinimumError(-0x80000000);
        if (n > 0x7fffffff) return const AboveMaximumError(0x7fffffff);
      }
      if (schema.minValue != null && n < schema.minValue!) {
        return BelowMinimumError(schema.minValue!);
      }
      if (schema.maxValue != null && n > schema.maxValue!) {
        return AboveMaximumError(schema.maxValue!);
      }
      return null;

    case FieldType.float_:
      final f = double.tryParse(trimmed);
      if (f == null) return const NotANumberError();
      // double.tryParse accepts NaN / Infinity / overflowing literals like
      // 1e309. Those slip past the min/max checks below (NaN compares false to
      // everything) and then crash jsonEncode when the override is sent to the
      // native FFI, so reject them up front.
      if (!f.isFinite) return const NotFiniteError();
      if (schema.numericDomain == FieldNumericDomain.finiteFloat32) {
        if (f < -3.4028234663852886e38) {
          return const BelowMinimumError(-3.4028234663852886e38);
        }
        if (f > 3.4028234663852886e38) {
          return const AboveMaximumError(3.4028234663852886e38);
        }
      }
      if (schema.minValue != null && f < schema.minValue!) {
        return BelowMinimumError(schema.minValue!);
      }
      if (schema.maxValue != null && f > schema.maxValue!) {
        return AboveMaximumError(schema.maxValue!);
      }
      return null;

    case FieldType.bool_:
      if (trimmed != 'true' && trimmed != 'false') {
        return const NotBoolError();
      }
      return null;

    case FieldType.string_:
      return null; // No further constraint at the GUI layer.

    case FieldType.enum_:
      if (!schema.enumValues.contains(trimmed)) {
        return NotInEnumError(schema.enumValues);
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
    FieldType.int_ => int.parse(trimmed),
    FieldType.float_ => double.parse(trimmed),
    FieldType.bool_ => trimmed == 'true',
    FieldType.string_ => trimmed,
    FieldType.enum_ => _enumBackingValue(schema, trimmed),
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
