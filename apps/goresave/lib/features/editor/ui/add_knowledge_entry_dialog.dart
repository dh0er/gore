import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';

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

class _AddKnowledgeEntryDialog extends StatefulWidget {
  const _AddKnowledgeEntryDialog({
    required this.catalog,
    required this.exclude,
  });

  final KnowledgeCatalog catalog;
  final Set<String> exclude;

  @override
  State<_AddKnowledgeEntryDialog> createState() =>
      _AddKnowledgeEntryDialogState();
}

// Fixed display order for knowledge categories.
const _kKnowledgeCategories = ['topic', 'choice', 'info'];

// Sentinel value for the "All" sidebar entry.
const _kAllCategory = '';

typedef _EntryGroup = ({String category, List<KnowledgeCatalogEntry> entries});

class _AddKnowledgeEntryDialogState extends State<_AddKnowledgeEntryDialog> {
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
      shown = available
          .where((e) => e.id.toLowerCase().contains(query))
          .toList();
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
      title: const Text('Add knowledge entry'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SizedBox(
        width: 720,
        height: 520,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _searchController,
              decoration: const InputDecoration(
                labelText: 'Search entries',
                prefixIcon: Icon(Icons.search),
                isDense: true,
              ),
              onChanged: (v) => setState(() {
                _query = v;
              }),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: groups.isEmpty
                  ? const Center(
                      child: Text('No knowledge entries available to add'))
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
                                    label: 'All (${available.length})',
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
                                      label:
                                          '${_cap(g.category)} (${g.entries.length})',
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
                              ? const Center(
                                  child: Text('No entries match'))
                              : ListView.builder(
                                  itemCount: shown.length,
                                  itemBuilder: (context, index) =>
                                      _entryTile(shown[index]),
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
          child: const Text('Cancel'),
        ),
      ],
    );
  }

  Widget _entryTile(KnowledgeCatalogEntry entry) {
    return ListTile(
      dense: true,
      leading: Icon(_iconForKnowledgeCategory(entry.category)),
      title: Text(
        entry.id,
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
