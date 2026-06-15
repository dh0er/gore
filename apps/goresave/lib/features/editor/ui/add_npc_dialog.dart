import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/npc_catalog.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';

/// Shows a picker dialog over [catalog] that lets the user choose an NPC.
///
/// Returns the selected NPC [id], or null if the dialog is dismissed.
/// [exclude] is a set of **lowercase** ids to hide (NPCs already present).
Future<String?> showAddNpcDialog(
  BuildContext context, {
  required NpcCatalog catalog,
  required Set<String> exclude,
}) {
  return showDialog<String>(
    context: context,
    builder: (_) => _AddNpcDialog(catalog: catalog, exclude: exclude),
  );
}

class _AddNpcDialog extends StatefulWidget {
  const _AddNpcDialog({required this.catalog, required this.exclude});

  final NpcCatalog catalog;
  final Set<String> exclude;

  @override
  State<_AddNpcDialog> createState() => _AddNpcDialogState();
}

typedef _NpcGroup = ({String category, List<NpcCatalogEntry> entries});

// Sentinel value for the "All" sidebar entry.
const _kAllCategory = '';

class _AddNpcDialogState extends State<_AddNpcDialog> {
  String _query = '';
  // Empty string means "All".
  String _selectedCategory = _kAllCategory;
  final TextEditingController _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  List<_NpcGroup> _buildGroups(List<NpcCatalogEntry> available) {
    final byCategory = <String, List<NpcCatalogEntry>>{};
    for (final entry in available) {
      byCategory.putIfAbsent(entry.category, () => []).add(entry);
    }
    // Collect distinct categories in the order they first appear (catalog is
    // already sorted by id so this gives a stable, predictable order).
    final seen = <String>{};
    final orderedCategories = <String>[];
    for (final entry in available) {
      if (seen.add(entry.category)) {
        orderedCategories.add(entry.category);
      }
    }
    return [
      for (final cat in orderedCategories)
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
    final List<NpcCatalogEntry> shown;
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
      title: const Text('Add NPC'),
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
                labelText: 'Search NPCs',
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
                  ? const Center(child: Text('No NPCs available to add'))
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
                                    icon: Icons.people_outline,
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
                                      icon: _iconForNpcCategory(g.category),
                                      label:
                                          '${g.category} (${g.entries.length})',
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
                              ? const Center(child: Text('No NPCs match'))
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

  Widget _entryTile(NpcCatalogEntry entry) {
    final theme = Theme.of(context);
    return ListTile(
      dense: true,
      leading: Icon(_iconForNpcCategory(entry.category)),
      title: Text(
        entry.id,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        entry.category,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: theme.textTheme.bodySmall,
      ),
      onTap: () => Navigator.of(context).pop(entry.id),
    );
  }
}

IconData _iconForNpcCategory(String category) {
  switch (category) {
    case 'human':
      return Icons.person_outline;
    case 'creature':
      return Icons.pets;
    default:
      return Icons.help_outline;
  }
}
