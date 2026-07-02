import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../loc/domain/loc_catalog_provider.dart';

/// Loc-id prefixes that count as dialog/bark lines. `info_`/`dia_` are
/// conversation lines; `gvl_`/`svm_` are barks. Everything else (e.g. `text_`,
/// `itfo_`) is excluded.
const Set<String> _kConversationPrefixes = {'info', 'dia'};
const Set<String> _kBarkPrefixes = {'gvl', 'svm'};

/// One row in the flattened dialog list: either a speaker group header or a
/// single dialog line under it.
sealed class DialogRow {
  const DialogRow();
}

/// A speaker group header row (e.g. "aaron", "g1hero").
class DialogGroupRow extends DialogRow {
  const DialogGroupRow({
    required this.speaker,
    required this.isBark,
    required this.lineCount,
  });

  /// 2nd underscore token of the ids in this group.
  final String speaker;

  /// Whether this is a bark group (`gvl_`/`svm_`) vs a conversation group.
  final bool isBark;

  /// Number of line rows in this group (before filtering).
  final int lineCount;
}

/// A single dialog line row.
class DialogLineRow extends DialogRow {
  const DialogLineRow({
    required this.id,
    required this.speaker,
    required this.isBark,
  });

  /// The (lowercased) loc id.
  final String id;

  /// The owning speaker group.
  final String speaker;

  /// Whether the owning group is a bark group (`gvl_`/`svm_`).
  final bool isBark;
}

/// A speaker group with its lines, used to build the flattened view.
class _DialogGroup {
  _DialogGroup({required this.speaker, required this.isBark});
  final String speaker;
  final bool isBark;
  final List<String> ids = [];
}

/// First underscore token of [id] (`info_aaron_001` -> `info`).
String _leadingToken(String id) {
  final i = id.indexOf('_');
  return i < 0 ? id : id.substring(0, i);
}

/// Second underscore token of [id], the speaker (`info_aaron_001` -> `aaron`,
/// `gvl_g1hero_x` -> `g1hero`). Falls back to the whole id when absent.
String _speakerToken(String id) {
  final first = id.indexOf('_');
  if (first < 0) return id;
  final second = id.indexOf('_', first + 1);
  if (second < 0) return id.substring(first + 1);
  return id.substring(first + 1, second);
}

/// Pure grouping logic behind [dialogRowsProvider]: builds the flattened list
/// of group-header + line rows from a loc catalog, conversation groups first,
/// then bark groups, each alphabetical by speaker. Lines within a group are
/// sorted by id.
List<DialogRow> buildDialogRows(Map<String, Map<String, String>> catalog) {
  if (catalog.isEmpty) return const [];

  // group by (isBark, speaker)
  final groups = <String, _DialogGroup>{};
  for (final id in catalog.keys) {
    final token = _leadingToken(id);
    final isConv = _kConversationPrefixes.contains(token);
    final isBark = _kBarkPrefixes.contains(token);
    if (!isConv && !isBark) continue;
    final speaker = _speakerToken(id);
    final key = '${isBark ? 1 : 0}:$speaker';
    (groups[key] ??= _DialogGroup(speaker: speaker, isBark: isBark)).ids.add(id);
  }

  final sorted = groups.values.toList()
    ..sort((a, b) {
      // conversation groups before bark groups, then alphabetical by speaker
      if (a.isBark != b.isBark) return a.isBark ? 1 : -1;
      return a.speaker.compareTo(b.speaker);
    });

  final rows = <DialogRow>[];
  for (final g in sorted) {
    g.ids.sort();
    rows.add(DialogGroupRow(
      speaker: g.speaker,
      isBark: g.isBark,
      lineCount: g.ids.length,
    ));
    for (final id in g.ids) {
      rows.add(DialogLineRow(id: id, speaker: g.speaker, isBark: g.isBark));
    }
  }
  return rows;
}

/// Derives the dialog browser model from [locCatalogProvider] via
/// [buildDialogRows].
final dialogRowsProvider = Provider<List<DialogRow>>((ref) {
  final catalog = ref.watch(locCatalogProvider).value ?? const {};
  return buildDialogRows(catalog);
});
