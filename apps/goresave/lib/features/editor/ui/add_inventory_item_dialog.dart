import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';

/// Maximum number of catalog entries shown after filtering to keep the dialog
/// responsive.
const int _kMaxDisplayedEntries = 100;

/// Dialog that lets the user pick an item from the bundled catalog and specify
/// a count to add to the inventory.
///
/// Returns [InventoryItemAdd] on confirmation, null on cancel.
///
/// [catalogOverride] is an optional future that provides a fake catalog for
/// widget tests; production callers leave it null to use [ItemCatalog.loadBundled].
class AddInventoryItemDialog extends StatefulWidget {
  const AddInventoryItemDialog({
    super.key,
    required this.existingItems,
    this.catalogOverride,
  });

  final List<PrivateInventoryItem> existingItems;
  final Future<ItemCatalog>? catalogOverride;

  @override
  State<AddInventoryItemDialog> createState() => _AddInventoryItemDialogState();
}

class _AddInventoryItemDialogState extends State<AddInventoryItemDialog> {
  String _query = '';
  ItemCatalogEntry? _selected;
  final TextEditingController _countController =
      TextEditingController(text: '1');
  String? _countError;

  @override
  void dispose() {
    _countController.dispose();
    super.dispose();
  }

  void _onCountChanged(String value) {
    final trimmed = value.trim();
    final parsed = int.tryParse(trimmed);
    setState(() {
      _countError = (parsed == null || parsed < 1) ? 'Must be ≥ 1' : null;
    });
  }

  bool get _canAdd {
    if (_selected == null) return false;
    final parsed = int.tryParse(_countController.text.trim());
    return parsed != null && parsed >= 1;
  }

  void _confirm() {
    final entry = _selected;
    if (entry == null) return;
    final parsed = int.tryParse(_countController.text.trim());
    if (parsed == null || parsed < 1) return;
    Navigator.of(context).pop(InventoryItemAdd(path: entry.path, count: parsed));
  }

  @override
  Widget build(BuildContext context) {
    final future = widget.catalogOverride ?? ItemCatalog.loadBundled();
    return AlertDialog(
      title: const Text('Add item'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SizedBox(
        width: 480,
        height: 520,
        child: FutureBuilder<ItemCatalog>(
          future: future,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Center(child: CircularProgressIndicator());
            }
            if (snapshot.hasError) {
              return Center(child: Text('Failed to load catalog: ${snapshot.error}'));
            }
            final catalog = snapshot.data!;
            // Paths of items already in the inventory.
            final existingPaths = {
              for (final item in widget.existingItems) item.path,
            };
            final query = _query.trim().toLowerCase();
            final filtered = catalog.entries
                .where((e) => !existingPaths.contains(e.path))
                .where((e) {
                  if (query.isEmpty) return true;
                  return e.id.toLowerCase().contains(query) ||
                      e.path.toLowerCase().contains(query);
                })
                .take(_kMaxDisplayedEntries)
                .toList();

            // Group by category.
            final byCategory = <ItemCategory, List<ItemCatalogEntry>>{};
            for (final entry in filtered) {
              byCategory
                  .putIfAbsent(itemCategoryFromId(entry.id), () => [])
                  .add(entry);
            }
            final groups = [
              for (final cat in ItemCategory.values)
                if (byCategory.containsKey(cat))
                  (category: cat, entries: byCategory[cat]!),
            ];

            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  decoration: const InputDecoration(
                    labelText: 'Search items',
                    prefixIcon: Icon(Icons.search),
                    isDense: true,
                  ),
                  onChanged: (v) => setState(() {
                    _query = v;
                    // Deselect if no longer visible.
                    if (_selected != null &&
                        !filtered.contains(_selected)) {
                      _selected = null;
                    }
                  }),
                ),
                const SizedBox(height: 8),
                if (_selected != null) ...[
                  // Count field + selection summary.
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          itemDisplayNameFromId(_selected!.id),
                          style: Theme.of(context).textTheme.bodyMedium,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const SizedBox(width: 8),
                      SizedBox(
                        width: 100,
                        child: TextField(
                          controller: _countController,
                          decoration: InputDecoration(
                            labelText: 'Count',
                            isDense: true,
                            errorText: _countError,
                          ),
                          keyboardType: TextInputType.number,
                          inputFormatters: [
                            FilteringTextInputFormatter.digitsOnly,
                          ],
                          onChanged: _onCountChanged,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                ],
                Expanded(
                  child: filtered.isEmpty
                      ? const Center(child: Text('No items match'))
                      : ListView.builder(
                          itemCount: _listItemCount(groups),
                          itemBuilder: (context, index) {
                            return _buildListItem(context, groups, index);
                          },
                        ),
                ),
              ],
            );
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(null),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _canAdd ? _confirm : null,
          child: const Text('Add'),
        ),
      ],
    );
  }

  int _listItemCount(
    List<({ItemCategory category, List<ItemCatalogEntry> entries})> groups,
  ) {
    int count = 0;
    for (final g in groups) {
      count += 1 + g.entries.length; // header + entries
    }
    return count;
  }

  Widget _buildListItem(
    BuildContext context,
    List<({ItemCategory category, List<ItemCatalogEntry> entries})> groups,
    int index,
  ) {
    // Flatten groups into header + entry rows.
    int offset = 0;
    for (final g in groups) {
      if (index == offset) {
        // Header row.
        return ListTile(
          dense: true,
          title: Text(
            '${g.category.label} (${g.entries.length})',
            style: Theme.of(context).textTheme.labelLarge,
          ),
        );
      }
      offset += 1;
      final localIndex = index - offset;
      if (localIndex < g.entries.length) {
        final entry = g.entries[localIndex];
        final isSelected = _selected == entry;
        final scheme = Theme.of(context).colorScheme;
        return ListTile(
          dense: true,
          selected: isSelected,
          selectedTileColor: scheme.primaryContainer,
          leading: const Icon(Icons.category_outlined),
          title: Text(
            itemDisplayNameFromId(entry.id),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            entry.id,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          onTap: () => setState(() {
            _selected = isSelected ? null : entry;
          }),
        );
      }
      offset += g.entries.length;
    }
    // Should not happen.
    return const SizedBox.shrink();
  }
}
