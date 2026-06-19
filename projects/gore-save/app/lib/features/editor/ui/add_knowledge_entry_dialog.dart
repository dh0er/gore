import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// Shows a picker dialog over [catalog] that lets the user choose a knowledge
/// entry to add.
///
/// Returns the selected entry [id], or null if the dialog is dismissed.
/// [exclude] is a set of **lowercase** ids to hide (entries already present).
Future<String?> showAddKnowledgeEntryDialog(
  BuildContext context, {
  required KnowledgeCatalog catalog,
  required Set<String> exclude,
}) {
  return showDialog<String>(
    context: context,
    builder: (_) =>
        _AddKnowledgeEntryDialog(catalog: catalog, exclude: exclude),
  );
}

class _AddKnowledgeEntryDialog extends ConsumerStatefulWidget {
  const _AddKnowledgeEntryDialog({
    required this.catalog,
    required this.exclude,
  });

  final KnowledgeCatalog catalog;
  final Set<String> exclude;

  @override
  ConsumerState<_AddKnowledgeEntryDialog> createState() =>
      _AddKnowledgeEntryDialogState();
}

// Fixed display order for knowledge categories.
const _kKnowledgeCategories = ['topic', 'choice', 'info'];

// Sentinel value for the "All" sidebar entry.
const _kAllCategory = '';

typedef _EntryGroup = ({String category, List<KnowledgeCatalogEntry> entries});

class _AddKnowledgeEntryDialogState
    extends ConsumerState<_AddKnowledgeEntryDialog> {
  String _query = '';
  // Empty string means "All".
  String _selectedCategory = _kAllCategory;
  final TextEditingController _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  List<_EntryGroup> _buildGroups(List<KnowledgeCatalogEntry> available) {
    final byCategory = <String, List<KnowledgeCatalogEntry>>{};
    for (final entry in available) {
      byCategory.putIfAbsent(entry.category, () => []).add(entry);
    }
    return [
      for (final cat in _kKnowledgeCategories)
        if (byCategory.containsKey(cat))
          (category: cat, entries: byCategory[cat]!),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final catalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};

    final available = widget.catalog.entries
        .where((e) => !widget.exclude.contains(e.id.toLowerCase()))
        .toList();
    final groups = _buildGroups(available);

    final selectedCategory = _selectedCategory;

    // Right-pane entries: a non-empty query searches the whole catalog;
    // "All" (_kAllCategory) shows everything; otherwise filter by category.
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;
    final List<KnowledgeCatalogEntry> shown;
    if (searching) {
      shown = available.where((e) {
        final name = localizedGameName(catalog, lang, e.id) ?? e.id;
        return e.id.toLowerCase().contains(query) ||
            name.toLowerCase().contains(query);
      }).toList();
    } else if (selectedCategory == _kAllCategory) {
      shown = available;
    } else {
      shown = groups
              .where((g) => g.category == selectedCategory)
              .firstOrNull
              ?.entries ??
          const [];
    }

    return AlertDialog(
      title: Text(l10n.addKnowledgeEntryDialogTitle),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SizedBox(
        width: 720,
        height: 520,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _searchController,
              decoration: InputDecoration(
                labelText: l10n.searchEntries,
                prefixIcon: const Icon(Icons.search),
                isDense: true,
              ),
              onChanged: (v) => setState(() {
                _query = v;
              }),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: groups.isEmpty
                  ? Center(child: Text(l10n.noKnowledgeEntriesAvailableToAdd))
                  : Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        SizedBox(
                          width: 200,
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              color: theme.colorScheme.surfaceContainerLow,
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: SingleChildScrollView(
                              padding:
                                  const EdgeInsets.symmetric(vertical: 6),
                              child: Column(
                                children: [
                                  SidebarTile(
                                    icon: Icons.list_outlined,
                                    label: l10n.allWithCount(available.length),
                                    selected: !searching &&
                                        selectedCategory == _kAllCategory,
                                    onTap: () => setState(() {
                                      _selectedCategory = _kAllCategory;
                                      _query = '';
                                      _searchController.clear();
                                    }),
                                  ),
                                  for (final g in groups)
                                    SidebarTile(
                                      icon:
                                          _iconForKnowledgeCategory(g.category),
                                      label: l10n.categoryWithCount(
                                          _cap(g.category), g.entries.length),
                                      selected: !searching &&
                                          g.category == selectedCategory,
                                      onTap: () => setState(() {
                                        _selectedCategory = g.category;
                                        _query = '';
                                        _searchController.clear();
                                      }),
                                    ),
                                ],
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 16),
                        Expanded(
                          child: shown.isEmpty
                              ? Center(child: Text(l10n.noEntriesMatch))
                              : ListView.builder(
                                  itemCount: shown.length,
                                  itemBuilder: (context, index) =>
                                      _entryTile(shown[index], catalog, lang),
                                ),
                        ),
                      ],
                    ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(null),
          child: Text(AppLocalizations.of(context).cancel),
        ),
      ],
    );
  }

  Widget _entryTile(
    KnowledgeCatalogEntry entry,
    Map<String, Map<String, String>> catalog,
    GameLang lang,
  ) {
    // Localized knowledge name from the id (lowercased); fall back to the raw
    // id when no extracted catalog entry exists.
    final name = localizedGameName(catalog, lang, entry.id) ?? entry.id;
    return ListTile(
      dense: true,
      leading: Icon(_iconForKnowledgeCategory(entry.category)),
      title: Text(
        name,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      onTap: () => Navigator.of(context).pop(entry.id),
    );
  }
}

String _cap(String s) => s.isEmpty ? s : '${s[0].toUpperCase()}${s.substring(1)}';

IconData _iconForKnowledgeCategory(String category) {
  switch (category) {
    case 'topic':
      return Icons.topic_outlined;
    case 'choice':
      return Icons.call_split;
    case 'info':
      return Icons.info_outline;
    default:
      return Icons.help_outline;
  }
}
