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
/// Mirrors gore-cli's `validate_mod_name`: the name becomes a single directory
/// component under the export folder (and an entry prefix inside the .zip), so
/// it must not contain path separators, the `..` parent reference, or control
/// characters (a newline could also terminate a comment in the generated Lua
/// and inject code). Returns null when valid, else a [ModNameError].
ModNameError? validateModName(String name) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return ModNameError.required;
  if (trimmed.runes.any((r) => r < 0x20 || r == 0x7f)) {
    return ModNameError.controlCharacters;
  }
  if (trimmed.contains('/') || trimmed.contains('\\')) {
    return ModNameError.pathSeparators;
  }
  if (trimmed == '.' || trimmed == '..') {
    return ModNameError.notAFolderName;
  }
  return null;
}
