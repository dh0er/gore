import 'dart:convert';

/// A mod-name validation failure, decoupled from any language. The UI maps each
/// case to a localized message via `AppLocalizations` (see `modNameErrorText`).
enum ModNameError {
  required,
  controlCharacters,
  pathSeparators,
  notAFolderName,
}

/// Validate a mod name before it is appended to a user-chosen output path.
///
/// Mirrors gore-cli's `validate_mod_name`: the name becomes a portable single
/// directory component under the export folder (and an entry prefix inside the
/// .zip). `.value-minis` is reserved for value-build intermediates.
/// Returns null when valid, else a [ModNameError].
ModNameError? validateModName(String name) {
  if (name.trim().isEmpty) return ModNameError.required;
  if (name.runes.any((r) => r < 0x20 || (r >= 0x7f && r <= 0x9f))) {
    return ModNameError.controlCharacters;
  }
  if (name.contains('/') || name.contains('\\')) {
    return ModNameError.pathSeparators;
  }
  if (name == '.' ||
      name == '..' ||
      name.toLowerCase() == '.value-minis' ||
      utf8.encode(name).length > 198 ||
      name.endsWith(' ') ||
      name.endsWith('.') ||
      RegExp(r'[:<>"|?*]').hasMatch(name) ||
      _isWindowsDeviceName(name)) {
    return ModNameError.notAFolderName;
  }
  return null;
}

bool _isWindowsDeviceName(String name) {
  final stem = name.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final folded = stem.toUpperCase();
  if (const {
    'CON',
    'PRN',
    'AUX',
    'NUL',
    r'CLOCK$',
    r'CONIN$',
    r'CONOUT$',
  }.contains(folded)) {
    return true;
  }
  return RegExp(r'^(COM|LPT)[1-9¹²³]$').hasMatch(folded);
}

/// Plain-English message for a [ModNameError], for the rare path with no
/// BuildContext (the export safety net). The UI normally shows the localized
/// version via `AppLocalizations`.
String modNameErrorEnglish(ModNameError error) {
  switch (error) {
    case ModNameError.required:
      return 'Enter a mod name.';
    case ModNameError.controlCharacters:
      return 'The mod name must not contain control characters.';
    case ModNameError.pathSeparators:
      return 'The mod name must not contain "/" or "\\".';
    case ModNameError.notAFolderName:
      return 'The mod name is not a valid folder name.';
  }
}
