/// Validate a mod name before it is appended to a user-chosen output path.
///
/// Mirrors gore-cli's `validate_mod_name`: the name becomes a single directory
/// component under the export folder (and an entry prefix inside the .zip), so
/// it must not contain path separators, the `..` parent reference, or control
/// characters (a newline could also terminate a comment in the generated Lua
/// and inject code). Returns null when valid, else a short error message.
String? validateModName(String name) {
  final trimmed = name.trim();
  if (trimmed.isEmpty) return 'Required';
  if (trimmed.runes.any((r) => r < 0x20 || r == 0x7f)) {
    return 'Must not contain control characters';
  }
  if (trimmed.contains('/') || trimmed.contains('\\')) {
    return 'Must not contain path separators';
  }
  if (trimmed == '.' || trimmed == '..') {
    return 'Not a valid folder name';
  }
  return null;
}
