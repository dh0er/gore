import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';

/// Dialog that lets the user pick an item from the bundled catalog and specify
/// a count to add to the inventory.
///
/// Returns [InventoryItemAdd] on confirmation, null on cancel. Items already in
/// the inventory are excluded. The full catalog is browsable via a category
/// sidebar; the search box filters across all categories.
///
/// [catalogOverride] is an optional future that provides a fake catalog for
/// widget tests; production callers leave it null to use
/// [ItemCatalog.loadBundled].
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

typedef _CatalogGroup = ({ItemCategory category, List<ItemCatalogEntry> entries});

class _AddInventoryItemDialogState extends State<AddInventoryItemDialog> {
  String _query = '';
  ItemCategory? _selectedCategory;
  ItemCatalogEntry? _selected;
  final TextEditingController _searchController = TextEditingController();
  final TextEditingController _countController =
      TextEditingController(text: '1');
  String? _countError;
  // Created once: a fresh future per build would reset the FutureBuilder
  // (spinner flash) on every setState.
  late final Future<ItemCatalog> _catalogFuture =
      widget.catalogOverride ?? ItemCatalog.loadBundled();

  @override
  void dispose() {
    _searchController.dispose();
    _countController.dispose();
    super.dispose();
  }

  void _onCountChanged(String value) {
    final parsed = int.tryParse(value.trim());
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

  List<_CatalogGroup> _group(List<ItemCatalogEntry> entries) {
    final byCategory = <ItemCategory, List<ItemCatalogEntry>>{};
    for (final entry in entries) {
      byCategory.putIfAbsent(itemCategoryFromId(entry.id), () => []).add(entry);
    }
    return [
      for (final cat in ItemCategory.values)
        if (byCategory.containsKey(cat)) (category: cat, entries: byCategory[cat]!),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AlertDialog(
      title: const Text('Add item'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SizedBox(
        width: 720,
        height: 520,
        child: FutureBuilder<ItemCatalog>(
          future: _catalogFuture,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Center(child: CircularProgressIndicator());
            }
            if (snapshot.hasError) {
              return Center(
                child: Text('Failed to load catalog: ${snapshot.error}'),
              );
            }
            final catalog = snapshot.data!;
            final existingPaths = {
              for (final item in widget.existingItems) item.path,
            };
            final available = catalog.entries
                .where((e) => !existingPaths.contains(e.path))
                .toList();
            final groups = _group(available);

            // Resolve the selected category (fall back to first available).
            var selectedCat = _selectedCategory;
            if (groups.every((g) => g.category != selectedCat)) {
              selectedCat = groups.isEmpty ? null : groups.first.category;
            }

            // Right-pane entries: a non-empty query searches the whole catalog;
            // an empty query shows the selected category.
            final query = _query.trim().toLowerCase();
            final searching = query.isNotEmpty;
            final List<ItemCatalogEntry> shown;
            if (searching) {
              shown = available.where((e) {
                return e.id.toLowerCase().contains(query) ||
                    e.path.toLowerCase().contains(query);
              }).toList();
            } else {
              shown = groups
                      .where((g) => g.category == selectedCat)
                      .firstOrNull
                      ?.entries ??
                  const [];
            }

            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  controller: _searchController,
                  decoration: const InputDecoration(
                    labelText: 'Search items',
                    prefixIcon: Icon(Icons.search),
                    isDense: true,
                  ),
                  onChanged: (v) => setState(() {
                    _query = v;
                    if (_selected != null && !shownContains(_selected!, v)) {
                      // Deselect if the selection scrolls out of the result set.
                      _selected = null;
                    }
                  }),
                ),
                const SizedBox(height: 8),
                if (_selected != null) ...[
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          itemDisplayNameFromId(_selected!.id),
                          style: theme.textTheme.bodyMedium,
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
                  child: groups.isEmpty
                      ? const Center(child: Text('No items available to add'))
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
                                      for (final g in groups)
                                        SidebarTile(
                                          icon: iconForItemCategory(g.category),
                                          label:
                                              '${g.category.label} (${g.entries.length})',
                                          selected:
                                              !searching && g.category == selectedCat,
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
                                  ? const Center(child: Text('No items match'))
                                  : ListView.builder(
                                      itemCount: shown.length,
                                      itemBuilder: (context, index) =>
                                          _entryTile(theme, shown[index]),
                                    ),
                            ),
                          ],
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

  /// Whether [entry] would still be visible for the given raw query string.
  bool shownContains(ItemCatalogEntry entry, String rawQuery) {
    final q = rawQuery.trim().toLowerCase();
    if (q.isEmpty) return itemCategoryFromId(entry.id) == _selectedCategory;
    return entry.id.toLowerCase().contains(q) ||
        entry.path.toLowerCase().contains(q);
  }

  Widget _entryTile(ThemeData theme, ItemCatalogEntry entry) {
    final isSelected = _selected == entry;
    return ListTile(
      dense: true,
      selected: isSelected,
      selectedTileColor: theme.colorScheme.primaryContainer,
      leading: Icon(iconForItemCategory(itemCategoryFromId(entry.id))),
      title: Text(
        itemDisplayNameFromId(entry.id),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(entry.id, maxLines: 1, overflow: TextOverflow.ellipsis),
      onTap: () => setState(() {
        _selected = isSelected ? null : entry;
      }),
    );
  }
}
