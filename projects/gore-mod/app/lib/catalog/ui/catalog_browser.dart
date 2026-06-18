import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../domain/catalog_provider.dart';
import '../domain/item_categories.dart';
import '../domain/item_entry.dart';
import 'sidebar_tile.dart';

/// Category-grouped, searchable item browser.
/// Calls [onItemSelected] when the user taps an item.
class CatalogBrowser extends ConsumerStatefulWidget {
  const CatalogBrowser({super.key, required this.onItemSelected, this.selected});

  final void Function(CatalogItem) onItemSelected;
  final CatalogItem? selected;

  @override
  ConsumerState<CatalogBrowser> createState() => _CatalogBrowserState();
}

class _CatalogBrowserState extends ConsumerState<CatalogBrowser> {
  String _query = '';
  ItemCategory? _selectedCategory;
  final TextEditingController _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final catalogAsync = ref.watch(catalogProvider);
    return catalogAsync.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error:   (e, _) => Center(child: Text('Failed to load catalog: $e')),
      data:    (items) => _buildBrowser(context, items),
    );
  }

  Widget _buildBrowser(BuildContext context, List<CatalogItem> items) {
    final theme = Theme.of(context);
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;

    // Filter
    final filtered = searching
        ? items.where((i) => i.id.toLowerCase().contains(query) ||
                             i.displayName.toLowerCase().contains(query)).toList()
        : items;

    final groups = groupCatalogItems(filtered);

    // Resolve selected category
    var selectedCat = _selectedCategory;
    if (groups.every((g) => g.category != selectedCat)) {
      selectedCat = groups.isEmpty ? null : groups.first.category;
    }

    final shownItems = searching
        ? filtered
        : (groups.where((g) => g.category == selectedCat).firstOrNull?.items ?? []);

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _searchController,
            decoration: const InputDecoration(
              labelText: 'Search items',
              prefixIcon: Icon(Icons.search),
              isDense: true,
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          child: groups.isEmpty
              ? const Center(child: Text('No items match'))
              : Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!searching) SizedBox(
                      width: 180,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: theme.colorScheme.surfaceContainerLow,
                        ),
                        child: ListView(
                          padding: const EdgeInsets.symmetric(vertical: 6),
                          children: [
                            for (final g in groups)
                              SidebarTile(
                                icon:     iconForItemCategory(g.category),
                                label:    '${g.category.label} (${g.items.length})',
                                selected: g.category == selectedCat,
                                onTap: () => setState(() {
                                  _selectedCategory = g.category;
                                }),
                              ),
                          ],
                        ),
                      ),
                    ),
                    const VerticalDivider(width: 1),
                    Expanded(
                      child: ListView.builder(
                        itemCount: shownItems.length,
                        itemBuilder: (context, index) {
                          final item = shownItems[index];
                          final isSelected = widget.selected?.id == item.id;
                          return ListTile(
                            dense: true,
                            selected: isSelected,
                            selectedTileColor: theme.colorScheme.primaryContainer,
                            leading: Icon(iconForItemCategory(itemCategoryFromId(item.id))),
                            title: Text(item.displayName, maxLines: 1, overflow: TextOverflow.ellipsis),
                            subtitle: Text(item.id, maxLines: 1, overflow: TextOverflow.ellipsis),
                            onTap: () => widget.onItemSelected(item),
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
