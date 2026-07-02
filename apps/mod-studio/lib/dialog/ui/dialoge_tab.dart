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

/// Sidebar display label for a raw speaker token (`aaron` -> `Aaron`).
String _speakerLabel(String speaker) {
  if (speaker.isEmpty) return '(unknown)';
  return speaker[0].toUpperCase() + speaker.substring(1);
}

class _DialogBrowserState extends ConsumerState<_DialogBrowser> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  /// Key of the speaker group shown in the line list while not searching
  /// (see [_groupKey]). Null / vanished keys fall back to the first group.
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
    final allRows = ref.watch(dialogRowsProvider);
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;
    final editedIds = ref.watch(locEditsProvider).edits;

    final groups = allRows.whereType<DialogGroupRow>().toList();

    // Resolve selected group; fall back to the first group when the stored
    // selection vanished (or nothing was selected yet).
    var selectedKey = _selectedGroupKey;
    if (!groups.any((g) => _groupKey(g.isBark, g.speaker) == selectedKey)) {
      selectedKey = groups.isEmpty
          ? null
          : _groupKey(groups.first.isBark, groups.first.speaker);
    }

    // Searching: flat cross-group hit list. Otherwise: the selected group's lines.
    final lines = allRows.whereType<DialogLineRow>();
    final shownLines = searching
        ? lines.where((l) => _matches(l.id, query, catalog, editedIds)).toList()
        : lines
            .where((l) => _groupKey(l.isBark, l.speaker) == selectedKey)
            .toList();

    final lang = gameLangByCode(ref.watch(localeProvider));
    final selectedId = ref.watch(_selectedDialogIdProvider);

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
                                  selected:
                                      _groupKey(g.isBark, g.speaker) ==
                                          selectedKey,
                                  onTap: () => setState(() {
                                    _selectedGroupKey =
                                        _groupKey(g.isBark, g.speaker);
                                  }),
                                ),
                            ],
                          ),
                        ),
                      ),
                    const VerticalDivider(width: 1),
                    Expanded(
                      child: shownLines.isEmpty
                          ? const Center(child: Text('No dialog lines match'))
                          : ListView.builder(
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
                                  onTap: () => ref
                                      .read(_selectedDialogIdProvider.notifier)
                                      .state = line.id,
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
