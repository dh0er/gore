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

/// The single group-key encoding shared by [DialogGroupRow.groupKey] and
/// [DialogLineRow.groupKey]: `'${isBark ? 1 : 0}:$speaker'`.
String _encodeGroupKey(bool isBark, String speaker) =>
    '${isBark ? 1 : 0}:$speaker';

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

  /// Number of line rows in this group. When [buildDialogRows] was given an
  /// `onlyIds` filter, this counts only the included lines.
  final int lineCount;

  /// Stable key identifying this group; its line rows carry the same
  /// [DialogLineRow.groupKey].
  String get groupKey => _encodeGroupKey(isBark, speaker);
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

  /// Key of the owning group; equals that group's [DialogGroupRow.groupKey].
  String get groupKey => _encodeGroupKey(isBark, speaker);
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

/// Whether a (lowercased) loc id is a dialog/bark line by prefix — the same
/// per-entry test [buildDialogRows] applies. Used by the Changes tab so its
/// Dialoge count/filter cover exactly the ids the dialog browser can show
/// (item-name and other non-dialog loc edits are excluded).
bool isDialogLocId(String id) {
  final token = _leadingToken(id);
  return _kConversationPrefixes.contains(token) ||
      _kBarkPrefixes.contains(token);
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
///
/// When [onlyIds] is non-null, only catalog entries whose (lowercased) id is
/// in the set are considered — the restriction happens BEFORE grouping, so
/// group membership and [DialogGroupRow.lineCount] reflect only the filtered
/// lines. An empty set yields no rows.
List<DialogRow> buildDialogRows(
  Map<String, Map<String, String>> catalog, {
  Set<String>? onlyIds,
}) {
  if (catalog.isEmpty) return const [];

  // group by (isBark, speaker)
  final groups = <String, _DialogGroup>{};
  for (final id in catalog.keys) {
    if (onlyIds != null && !onlyIds.contains(id)) continue;
    if (!isDialogLocId(id)) continue;
    final isBark = _kBarkPrefixes.contains(_leadingToken(id));
    final speaker = _speakerToken(id);
    final key = _encodeGroupKey(isBark, speaker);
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

/// Identity-keyed memo around [buildDialogRows] for filtered (`onlyIds`)
/// views.
///
/// The filtered dialog browser rebuilds on every staged-edit change —
/// [locEditsProvider] emits per keystroke while editing — and each row build
/// scans the whole catalog (~43k ids). Both inputs are treated as immutable
/// values: callers keep the SAME instances while the content is unchanged
/// (the Changes tab content-compares its dialog-id set before swapping it),
/// so cheap identity comparison suffices as the cache key.
class DialogRowsMemo {
  Map<String, Map<String, String>>? _catalog;
  Set<String>? _onlyIds;
  List<DialogRow>? _rows;

  /// Rows for ([catalog], [onlyIds]); returns the previously built list
  /// instance when both arguments are identical to the last call's.
  List<DialogRow> rowsFor(
    Map<String, Map<String, String>> catalog,
    Set<String> onlyIds,
  ) {
    if (!identical(catalog, _catalog) || !identical(onlyIds, _onlyIds)) {
      _catalog = catalog;
      _onlyIds = onlyIds;
      _rows = buildDialogRows(catalog, onlyIds: onlyIds);
    }
    return _rows!;
  }
}

/// Derives the dialog browser model from [locCatalogProvider] via
/// [buildDialogRows].
final dialogRowsProvider = Provider<List<DialogRow>>((ref) {
  final catalog = ref.watch(locCatalogProvider).value ?? const {};
  return buildDialogRows(catalog);
});
