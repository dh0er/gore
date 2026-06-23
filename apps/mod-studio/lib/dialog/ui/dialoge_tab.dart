import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../../app/domain/ui_settings.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../loc/game_lang.dart';
import '../../loc/ui/lang_fields.dart';
import '../domain/dialog_catalog_provider.dart';

/// Currently selected dialog line (loc id), local to the Dialoge tab.
final _selectedDialogIdProvider = StateProvider<String?>((ref) => null);

/// Browse & edit the game's dialog / bark lines across languages. Edits are
/// staged into the shared [locEditsProvider].
class DialogeTab extends ConsumerWidget {
  const DialogeTab({super.key});

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
          children: const [
            SizedBox(width: 560, child: _DialogBrowser()),
            VerticalDivider(width: 1),
            Expanded(child: _DialogEditor()),
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
  const _DialogBrowser();

  @override
  ConsumerState<_DialogBrowser> createState() => _DialogBrowserState();
}

/// Stable key for a speaker group: `'${isBark}:${speaker}'`.
String _groupKey(bool isBark, String speaker) => '$isBark:$speaker';

class _DialogBrowserState extends ConsumerState<_DialogBrowser> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  /// Keys of groups the user has expanded. Empty = everything collapsed.
  final Set<String> _expanded = <String>{};

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _toggleGroup(DialogGroupRow row) {
    final key = _groupKey(row.isBark, row.speaker);
    setState(() {
      if (!_expanded.remove(key)) _expanded.add(key);
    });
  }

  /// Collapse-aware view (used when no search query is active): always emit
  /// group headers, but emit a group's line rows only when it is expanded.
  List<DialogRow> _collapsedRows(List<DialogRow> rows) {
    final out = <DialogRow>[];
    var currentExpanded = false;
    for (final row in rows) {
      if (row is DialogGroupRow) {
        currentExpanded = _expanded.contains(_groupKey(row.isBark, row.speaker));
        out.add(row);
      } else if (row is DialogLineRow && currentExpanded) {
        out.add(row);
      }
    }
    return out;
  }

  /// Whether [id]'s catalog entry matches [query] by id substring or by any of
  /// its set values containing the query (both already lowercased).
  bool _matches(
    String id,
    String query,
    Map<String, Map<String, String>> catalog,
  ) {
    if (id.contains(query)) return true;
    final entry = catalog[id];
    if (entry == null) return false;
    for (final v in entry.values) {
      if (v.toLowerCase().contains(query)) return true;
    }
    return false;
  }

  /// Build the filtered, flattened row list. Group headers are kept only when
  /// the group has at least one matching line under the active query.
  List<DialogRow> _filteredRows(
    List<DialogRow> rows,
    String query,
    Map<String, Map<String, String>> catalog,
  ) {
    if (query.isEmpty) return rows;
    final out = <DialogRow>[];
    DialogGroupRow? pendingHeader;
    var headerHasChildren = false;
    for (final row in rows) {
      if (row is DialogGroupRow) {
        pendingHeader = row;
        headerHasChildren = false;
      } else if (row is DialogLineRow) {
        if (!_matches(row.id, query, catalog)) continue;
        if (pendingHeader != null && !headerHasChildren) {
          out.add(pendingHeader);
          headerHasChildren = true;
        }
        out.add(row);
      }
    }
    return out;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final catalog = ref.watch(locCatalogProvider).value ?? const {};
    final allRows = ref.watch(dialogRowsProvider);
    final query = _query.trim().toLowerCase();
    // With a search query active, show matching lines (groups effectively
    // expanded). With no query, apply the default-collapsed group logic.
    final rows = query.isEmpty
        ? _collapsedRows(allRows)
        : _filteredRows(allRows, query, catalog);

    final lang = gameLangByCode(ref.watch(localeProvider));
    final selectedId = ref.watch(_selectedDialogIdProvider);
    final editedIds = ref.watch(locEditsProvider).edits;

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _searchController,
            decoration: const InputDecoration(
              labelText: 'Search dialog (id or text)',
              prefixIcon: Icon(Icons.search),
              isDense: true,
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          child: rows.isEmpty
              ? const Center(child: Text('No dialog lines match'))
              : ListView.builder(
                  itemCount: rows.length,
                  itemBuilder: (context, index) {
                    final row = rows[index];
                    if (row is DialogGroupRow) {
                      // When searching, matching groups are shown expanded.
                      final expanded = query.isNotEmpty ||
                          _expanded
                              .contains(_groupKey(row.isBark, row.speaker));
                      return _GroupHeader(
                        row: row,
                        expanded: expanded,
                        onTap: () => _toggleGroup(row),
                      );
                    }
                    final line = row as DialogLineRow;
                    final preview =
                        resolveGameText(catalog, line.id, lang) ??
                            _previewFor(line.id, catalog);
                    return ListTile(
                      dense: true,
                      selected: line.id == selectedId,
                      selectedTileColor: theme.colorScheme.primaryContainer,
                      leading: editedIds.containsKey(line.id)
                          ? Icon(Icons.circle,
                              size: 10, color: theme.colorScheme.primary)
                          : const SizedBox(width: 10),
                      title: Text(line.id,
                          maxLines: 1, overflow: TextOverflow.ellipsis),
                      subtitle: preview == null
                          ? null
                          : Text(preview,
                              maxLines: 1, overflow: TextOverflow.ellipsis),
                      onTap: () => ref
                          .read(_selectedDialogIdProvider.notifier)
                          .state = line.id,
                    );
                  },
                ),
        ),
      ],
    );
  }
}

/// A short text preview for a line: its first non-empty set value, if any.
String? _previewFor(String id, Map<String, Map<String, String>> catalog) {
  final entry = catalog[id];
  if (entry == null) return null;
  for (final v in entry.values) {
    if (v.trim().isNotEmpty) return v;
  }
  return null;
}

class _GroupHeader extends StatelessWidget {
  const _GroupHeader({
    required this.row,
    required this.expanded,
    required this.onTap,
  });
  final DialogGroupRow row;
  final bool expanded;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      child: Container(
        color: theme.colorScheme.surfaceContainerHigh,
        padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
        child: Row(
          children: [
            Icon(
              expanded ? Icons.expand_more : Icons.chevron_right,
              size: 18,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 4),
            Icon(
              row.isBark ? Icons.campaign_outlined : Icons.forum_outlined,
              size: 16,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                row.speaker.isEmpty ? '(unknown)' : row.speaker,
                style: theme.textTheme.titleSmall,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            Text(
              row.isBark ? 'bark · ${row.lineCount}' : '${row.lineCount}',
              style: theme.textTheme.labelSmall
                  ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
            ),
          ],
        ),
      ),
    );
  }
}

class _DialogEditor extends ConsumerWidget {
  const _DialogEditor();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final id = ref.watch(_selectedDialogIdProvider);
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
