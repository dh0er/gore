import 'game_lang.dart';

/// Localized-text resolvers for progression ids (quest names, dialog-knowledge
/// entries) whose save id does NOT equal the loc-catalog key. The transforms
/// here were validated against a real save: see the `quest_coverage` /
/// `loc_coverage` examples in goresave_core.
///
/// [catalog] is the loaded loc_catalog (`id -> {set -> text}`), keys lowercased.
/// All functions return null when nothing resolves so callers fall back to the
/// raw id.

/// Sorted catalog keys, memoized per catalog instance (the provider hands the
/// same map identity until a re-extraction). Used for prefix lookups.
final _sortedKeysCache = Expando<List<String>>();

List<String> _sortedKeys(Map<String, Map<String, String>> catalog) {
  final cached = _sortedKeysCache[catalog];
  if (cached != null) return cached;
  final keys = catalog.keys.toList()..sort();
  _sortedKeysCache[catalog] = keys;
  return keys;
}

/// First index in [keys] whose value is >= [needle] (lower_bound).
int _lowerBound(List<String> keys, String needle) {
  var lo = 0;
  var hi = keys.length;
  while (lo < hi) {
    final mid = (lo + hi) >> 1;
    if (keys[mid].compareTo(needle) < 0) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  return lo;
}

/// True when [s] contains a run of >= 4 digits — the mark of an internal
/// dialog-node hash (e.g. `Topic_Diego_209799`) that has no catalog text.
bool _hasLongNumber(String s) {
  var run = 0;
  for (final c in s.codeUnits) {
    if (c >= 0x30 && c <= 0x39) {
      if (++run >= 4) return true;
    } else {
      run = 0;
    }
  }
  return false;
}

/// CamelCase / mixed → snake_case, also splitting letter<->digit boundaries
/// (`Stt311` → `stt_311`) so ids match the catalog's spelling.
String _toSnake(String s) {
  final out = StringBuffer();
  String? last() => out.isEmpty ? null : out.toString().substring(out.length - 1);
  for (final ch in s.split('')) {
    if (ch == '_' || ch == '-') {
      if (last() != '_') out.write('_');
      continue;
    }
    final isUpper = ch.compareTo('A') >= 0 && ch.compareTo('Z') <= 0;
    final isLower = ch.compareTo('a') >= 0 && ch.compareTo('z') <= 0;
    final isDigit = ch.compareTo('0') >= 0 && ch.compareTo('9') <= 0;
    if (isUpper) {
      final l = last();
      if (l != null && l != '_') out.write('_');
      out.write(ch.toLowerCase());
    } else if (isDigit) {
      final l = last();
      if (l != null && l.compareTo('a') >= 0 && l.compareTo('z') <= 0) out.write('_');
      out.write(ch);
    } else {
      final l = last();
      if (isLower && l != null && l.compareTo('0') >= 0 && l.compareTo('9') <= 0) {
        out.write('_');
      }
      out.write(ch);
    }
  }
  return out.toString().replaceAll(RegExp(r'^_+|_+$'), '');
}

/// Resolve [stem] to localized text: exact key first, else the first catalog
/// key starting with `stem_` (the variant-suffixed dialog lines).
String? _resolveStem(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String stem,
) {
  final exact = resolveGameText(catalog, stem, lang);
  if (exact != null) return exact;
  final keys = _sortedKeys(catalog);
  final needle = '${stem}_';
  final i = _lowerBound(keys, needle);
  if (i < keys.length && keys[i].startsWith(needle)) {
    return resolveGameText(catalog, keys[i], lang);
  }
  return null;
}

/// Localized quest name for a quest class id, e.g.
/// `Quest_BanditsCamp_BANDITSTRUST` → `quest-banditscamp_banditstrust-name`.
/// Returns null for codex/bestiary pseudo-quests with no name string.
String? localizedQuestName(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String questId,
) {
  if (catalog.isEmpty || questId.isEmpty) return null;
  final body =
      (questId.startsWith('Quest_') ? questId.substring(6) : questId).toLowerCase();
  return resolveGameText(catalog, 'quest-$body-name', lang);
}

/// Localized text for a dialog-knowledge entry, or null. Handles:
///  - `Voiceline_<id>_AlkimiaLocalization` → exact inner id (single line)
///  - `Choice<Name>` / `Topic_<Name>` / `Info_*` → `info_<snake>` (+variant)
///  - purely numeric node ids → null (no catalog text)
String? localizedKnowledgeEntry(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String entry,
) {
  if (catalog.isEmpty || entry.isEmpty) return null;
  final lower = entry.toLowerCase();

  // Voiceline wrapper: strip prefix + trailing _AlkimiaLocalization.
  if (lower.startsWith('voiceline_')) {
    var inner = lower.substring('voiceline_'.length);
    const suffix = '_alkimialocalization';
    final i = inner.lastIndexOf(suffix);
    if (i >= 0) inner = inner.substring(0, i);
    return resolveGameText(catalog, inner, lang);
  }

  // Internal node hashes have no text.
  if (_hasLongNumber(entry)) return null;

  final snake = _toSnake(entry);
  final body = snake.startsWith('choice_')
      ? snake.substring('choice_'.length)
      : snake.startsWith('topic_')
      ? snake.substring('topic_'.length)
      : snake;
  final stems = <String>{snake, 'info_$body', 'dia_$body', 'info_$snake', 'dia_$snake'};
  for (final stem in stems) {
    final r = _resolveStem(catalog, lang, stem);
    if (r != null) return r;
  }
  return null;
}
