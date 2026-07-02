import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../../app/domain/ui_settings.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../loc/game_lang.dart';
import '../../loc/primary_set.dart';
import '../../loc/ui/lang_fields.dart';
import '../domain/dialog_catalog_provider.dart';

/// Currently selected dialog line (loc id), shared by all [DialogeTab]
/// instances (the main tab and filtered embeddings such as the Changes tab)
/// so the selection survives tab switches. The main tab owns clearing it;
/// filtered views must never write null here just because the id fell out of
/// their filter — they guard at view level instead (see [_DialogEditor]).
final selectedDialogIdProvider = StateProvider<String?>((ref) => null);

/// Browse & edit the game's dialog / bark lines across languages. Edits are
/// staged into the shared [locEditsProvider].
class DialogeTab extends ConsumerWidget {
  const DialogeTab({super.key, this.onlyIds});

  /// When non-null, the browser shows only dialog lines whose (lowercased)
  /// loc id is in this set — the same key space as [locEditsProvider] edit
  /// keys. The restriction applies before grouping, so the speaker sidebar
  /// only lists groups with at least one filtered line and counts reflect
  /// the filtered lines. Null (default) shows the full catalog.
  final Set<String>? onlyIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final catalogAsync = ref.watch(locCatalogProvider);
    return catalogAsync.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (e, _) => Center(child: Text('Failed to load localization: $e')),
      data: (catalog) {
        if (catalog.isEmpty) return const _EmptyHint();
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(width: 560, child: _DialogBrowser(onlyIds: onlyIds)),
            const VerticalDivider(width: 1),
            Expanded(child: _DialogEditor(onlyIds: onlyIds)),
          ],
        );
      },
    );
  }
}

class _EmptyHint extends StatelessWidget {
  const _EmptyHint();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.forum_outlined,
                size: 48, color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(height: 16),
            Text('No dialog lines yet', style: theme.textTheme.titleMedium),
            const SizedBox(height: 8),
            Text(
              'Extract the localization catalog first to browse and edit '
              'dialog and bark lines.',
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
            ),
          ],
        ),
      ),
    );
  }
}

class _DialogBrowser extends ConsumerStatefulWidget {
  const _DialogBrowser({this.onlyIds});

  /// See [DialogeTab.onlyIds].
  final Set<String>? onlyIds;

  @override
  ConsumerState<_DialogBrowser> createState() => _DialogBrowserState();
}

/// Sidebar display label for a raw speaker token (`aaron` -> `Aaron`).
String _speakerLabel(String speaker) {
  if (speaker.isEmpty) return '(unknown)';
  return speaker[0].toUpperCase() + speaker.substring(1);
}

class _DialogBrowserState extends ConsumerState<_DialogBrowser> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  /// Memoizes the filtered row build (see [DialogRowsMemo]) so per-keystroke
  /// rebuilds while editing inside the Changes tab don't re-scan the catalog.
  final DialogRowsMemo _rowsMemo = DialogRowsMemo();

  /// Key of the speaker group shown in the line list while not searching
  /// (see [DialogGroupRow.groupKey]). Null / vanished keys fall back to the
  /// selected line's group, then to the first group.
  String? _selectedGroupKey;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Whether [id]'s catalog entry matches [query] by id substring or by any of
  /// its set values containing the query (both already lowercased).
  bool _matches(
    String id,
    String query,
    Map<String, Map<String, String>> catalog,
    Map<String, Map<String, String>> edits,
  ) {
    if (id.contains(query)) return true;
    for (final v in catalog[id]?.values ?? const <String>[]) {
      if (v.toLowerCase().contains(query)) return true;
    }
    // Also match staged edits, so search agrees with the (edited) subtitles + what deploys.
    for (final v in edits[id]?.values ?? const <String>[]) {
      if (v.toLowerCase().contains(query)) return true;
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final catalog = ref.watch(locCatalogProvider).value ?? const {};
    // Filtered views derive their rows locally (memoized on input identity)
    // so the shared unfiltered provider path stays untouched (and
    // un-invalidated) for the main tab.
    final onlyIds = widget.onlyIds;
    final allRows = onlyIds == null
        ? ref.watch(dialogRowsProvider)
        : _rowsMemo.rowsFor(catalog, onlyIds);
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;
    final editedIds = ref.watch(locEditsProvider).edits;

    final groups = allRows.whereType<DialogGroupRow>().toList();
    final lines = allRows.whereType<DialogLineRow>();
    final selectedId = ref.watch(selectedDialogIdProvider);

    // Resolve selected group. When the stored selection is null or vanished,
    // restore from the still-selected editor line's group (this widget's state
    // dies on tab switches, the id provider doesn't; also covers clearing a
    // search after picking a hit), then fall back to the first group.
    var selectedKey = _selectedGroupKey;
    if (!groups.any((g) => g.groupKey == selectedKey)) {
      selectedKey = lines.firstWhereOrNull((l) => l.id == selectedId)?.groupKey ??
          (groups.isEmpty ? null : groups.first.groupKey);
    }

    // Searching: flat cross-group hit list. Otherwise: the selected group's lines.
    final shownLines = searching
        ? lines.where((l) => _matches(l.id, query, catalog, editedIds)).toList()
        : lines.where((l) => l.groupKey == selectedKey).toList();

    final lang = gameLangByCode(ref.watch(localeProvider));

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              labelText: 'Search dialog (id or text)',
              prefixIcon: const Icon(Icons.search),
              isDense: true,
              suffixIcon: _query.isEmpty
                  ? null
                  : IconButton(
                      icon: const Icon(Icons.clear),
                      tooltip: 'Clear',
                      onPressed: () {
                        _searchController.clear();
                        setState(() => _query = '');
                      },
                    ),
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          child: groups.isEmpty
              ? const Center(child: Text('No dialog lines match'))
              : Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!searching)
                      SizedBox(
                        width: 230,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerLow,
                          ),
                          child: ListView(
                            padding: const EdgeInsets.symmetric(vertical: 6),
                            children: [
                              for (final g in groups)
                                SidebarTile(
                                  icon: g.isBark
                                      ? Icons.campaign_outlined
                                      : Icons.forum_outlined,
                                  label: l10n.categoryWithCount(
                                      _speakerLabel(g.speaker), g.lineCount),
                                  selected: g.groupKey == selectedKey,
                                  onTap: () {
                                    // Don't leave a line from ANOTHER group
                                    // open in the editor pane: clear the
                                    // selection unless it belongs to the
                                    // tapped group.
                                    final selLine = selectedId == null
                                        ? null
                                        : lines.firstWhereOrNull(
                                            (l) => l.id == selectedId);
                                    if (selectedId != null &&
                                        selLine?.groupKey != g.groupKey) {
                                      ref
                                          .read(selectedDialogIdProvider
                                              .notifier)
                                          .state = null;
                                    }
                                    setState(() {
                                      _selectedGroupKey = g.groupKey;
                                    });
                                  },
                                ),
                            ],
                          ),
                        ),
                      ),
                    // The divider belongs to the sidebar — hide both during a
                    // search (matching the audio SFX split view).
                    if (!searching) const VerticalDivider(width: 1),
                    Expanded(
                      child: shownLines.isEmpty
                          ? const Center(child: Text('No dialog lines match'))
                          : ListView.builder(
                              // Reset scroll to top when the shown collection
                              // changes identity (group switch, search toggle).
                              key: searching
                                  ? const ValueKey('search')
                                  : ValueKey(selectedKey),
                              itemCount: shownLines.length,
                              itemBuilder: (context, index) {
                                final line = shownLines[index];
                                // Match the editor field exactly: the staged edit, else the value
                                // in THIS language's target set — with NO English fallback. So the
                                // list preview shows what editing/deploy actually change; a line
                                // empty in the current language shows no preview (like its editor
                                // field), instead of misleading English copy.
                                final set =
                                    primarySetFor(catalog, line.id, lang);
                                final stagedText = editedIds[line.id]?[set];
                                final langValue =
                                    catalog[line.id.toLowerCase()]?[set];
                                final preview = stagedText ??
                                    ((langValue != null &&
                                            langValue.trim().isNotEmpty)
                                        ? langValue
                                        : null);
                                return ListTile(
                                  dense: true,
                                  selected: line.id == selectedId,
                                  selectedTileColor:
                                      theme.colorScheme.primaryContainer,
                                  leading: editedIds.containsKey(line.id)
                                      ? Icon(Icons.circle,
                                          size: 10,
                                          color: theme.colorScheme.primary)
                                      : const SizedBox(width: 10),
                                  title: Text(line.id,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis),
                                  subtitle: preview == null
                                      ? null
                                      : Text(preview,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis),
                                  onTap: () {
                                    ref
                                        .read(
                                            selectedDialogIdProvider.notifier)
                                        .state = line.id;
                                    // Keep the sidebar in sync with the picked
                                    // line, so clearing a search lands on its
                                    // group (no-op in group view).
                                    setState(() =>
                                        _selectedGroupKey = line.groupKey);
                                  },
                                );
                              },
                            ),
                    ),
                  ],
                ),
        ),
      ],
    );
  }
}

class _DialogEditor extends ConsumerWidget {
  const _DialogEditor({this.onlyIds});

  /// See [DialogeTab.onlyIds].
  final Set<String>? onlyIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final selectedId = ref.watch(selectedDialogIdProvider);
    // View-level guard for filtered embeddings (the Changes tab): the shared
    // selection may point at a line OUTSIDE the filter — picked on the main
    // Dialoge tab, or its last staged edit was just removed. Show the
    // placeholder instead of an out-of-filter editor, but do NOT clear the
    // shared provider: the main tab owns that selection and keeps it.
    final filter = onlyIds;
    final id =
        (filter != null && selectedId != null && !filter.contains(selectedId))
            ? null
            : selectedId;
    if (id == null) {
      return Center(
        child: Text(
          'Select a dialog line to edit',
          style: theme.textTheme.bodyMedium
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
        ),
      );
    }
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.translate, size: 20),
            const SizedBox(width: 8),
            Expanded(
              child: Text(id, style: theme.textTheme.titleMedium),
            ),
            TextButton.icon(
              onPressed: () =>
                  ref.read(locEditsProvider.notifier).clearForId(id),
              icon: const Icon(Icons.undo, size: 18),
              label: const Text('Clear edits for this line'),
            ),
          ],
        ),
        const SizedBox(height: 12),
        LangFieldsEditor(locId: id),
      ],
    );
  }
}
