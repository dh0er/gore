import '../editor/domain/field_validator.dart';
import '../export/domain/mod_name.dart';
import 'app_localizations.dart';

/// Maps a language-agnostic [FieldError] to a localized message.
String fieldErrorText(AppLocalizations l10n, FieldError error) {
  return switch (error) {
    RequiredFieldError() => l10n.validationRequired,
    NotWholeNumberError() => l10n.validationMustBeWholeNumber,
    NotANumberError() => l10n.validationMustBeNumber,
    NotFiniteError() => l10n.validationMustBeFinite,
    BelowMinimumError(:final minValue) => l10n.validationMustBeAtLeast(
      _numText(minValue),
    ),
    AboveMaximumError(:final maxValue) => l10n.validationMustBeAtMost(
      _numText(maxValue),
    ),
    NotBoolError() => l10n.validationMustBeBool,
    NotInEnumError(:final allowed) => l10n.validationMustBeOneOf(
      allowed.join(', '),
    ),
  };
}

/// Maps a [ModNameError] to a localized message.
String modNameErrorText(AppLocalizations l10n, ModNameError error) {
  return switch (error) {
    ModNameError.required => l10n.modNameRequired,
    ModNameError.controlCharacters => l10n.modNameControlCharacters,
    ModNameError.pathSeparators => l10n.modNamePathSeparators,
    ModNameError.notAFolderName => l10n.modNameNotAFolderName,
  };
}

/// Render a numeric bound without a trailing `.0` for whole-number doubles,
/// matching the previous interpolation of `num` values.
String _numText(num n) {
  if (n is int) return n.toString();
  final text = n.toString();
  return text.endsWith('.0') ? text.substring(0, text.length - 2) : text;
}
