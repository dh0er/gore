import 'game_lang.dart';

/// The loc set to read AND write for `(locId, lang)`: the first set in `lang.locSets`
/// the catalog carries a non-empty value for, else `lang.locSets.first`. Read and write
/// use the same resolver so editing is WYSIWYG — you edit exactly the set
/// [resolveGameText] displays.
String primarySetFor(
  Map<String, Map<String, String>> catalog,
  String locId,
  GameLang lang,
) {
  final entry = catalog[locId.toLowerCase()];
  if (entry != null) {
    for (final set in lang.locSets) {
      final v = entry[set];
      if (v != null && v.trim().isNotEmpty) return set;
    }
  }
  return lang.locSets.first;
}
