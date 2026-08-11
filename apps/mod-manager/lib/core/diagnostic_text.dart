/// Sanitized, rune-bounded text suitable for visible and copyable diagnostics.
///
/// Controls become one collapsed space. This includes the complete Unicode
/// Bidi_Control set used by the Manager's startup diagnostics, so an untrusted
/// path/detail cannot visually reorder the localized explanation around it.
({String? value, bool truncated}) boundedDiagnosticText(
  String? raw,
  int maxRunes,
) {
  if (raw == null) return (value: null, truncated: false);
  final buffer = StringBuffer();
  var count = 0;
  var truncated = false;
  var previousWasSpace = false;
  for (final rune in raw.runes) {
    if (count == maxRunes) {
      truncated = true;
      break;
    }
    final isControl =
        rune < 0x20 ||
        (rune >= 0x7f && rune <= 0x9f) ||
        rune == 0x061c ||
        (rune >= 0x200e && rune <= 0x200f) ||
        rune == 0x2028 ||
        rune == 0x2029 ||
        (rune >= 0x202a && rune <= 0x202e) ||
        (rune >= 0x2066 && rune <= 0x2069);
    final isSpace = isControl || rune == 0x20;
    if (isSpace) {
      if (!previousWasSpace && buffer.isNotEmpty) buffer.write(' ');
      previousWasSpace = true;
    } else {
      buffer.writeCharCode(rune);
      previousWasSpace = false;
    }
    count++;
  }
  final value = buffer.toString().trim();
  return (value: value.isEmpty ? null : value, truncated: truncated);
}
