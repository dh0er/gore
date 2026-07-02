import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../app/domain/ui_settings.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/game_lang.dart';
import '../domain/catalog_provider.dart';
import '../domain/item_categories.dart';
import '../domain/item_entry.dart';
import 'sidebar_tile.dart';

/// Simple diacritic fold for sort keys (applied after lowercasing) so that
/// e.g. "Öllampe" sorts near "O" instead of after "Z". Deliberately minimal —
/// no intl/collator dependency.
const Map<String, String> _sortCharFold = {
  'ä': 'a', 'ö': 'o', 'ü': 'u', 'ß': 'ss',
  'à': 'a', 'á': 'a', 'â': 'a',
  'è': 'e', 'é': 'e', 'ê': 'e',
  'ì': 'i', 'í': 'i', 'î': 'i',
  'ò': 'o', 'ó': 'o', 'ô': 'o',
  'ù': 'u', 'ú': 'u', 'û': 'u',
};

String _localizedSortKey(String name) {
  final lower = name.toLowerCase();
  final buf = StringBuffer();
  for (final rune in lower.runes) {
    final ch = String.fromCharCode(rune);
    buf.write(_sortCharFold[ch] ?? ch);
  }
  return buf.toString();
}

/// Returns a copy of [items] sorted by the localized display name
/// (case-insensitive, diacritics folded), falling back to the class id as
/// tiebreaker. Decorate-sort-undecorate: [nameOf] is evaluated once per
/// item, not once per comparison.
List<CatalogItem> sortByLocalizedName(
    List<CatalogItem> items, String Function(CatalogItem) nameOf) {
  final decorated = [
    for (final item in items)
      (key: _localizedSortKey(nameOf(item)), item: item),
  ]..sort((a, b) {
      final c = a.key.compareTo(b.key);
      return c != 0 ? c : a.item.id.compareTo(b.item.id);
    });
  return [for (final d in decorated) d.item];
}

/// Category-grouped, searchable item browser.
/// Calls [onItemSelected] when the user taps an item.
class CatalogBrowser extends ConsumerStatefulWidget {
  const CatalogBrowser({
    super.key,
    required this.onItemSelected,
    this.selected,
    this.onlyIds,
  });

  final void Function(CatalogItem) onItemSelected;
  final CatalogItem? selected;

  /// When non-null, restricts the item universe to these class ids before
  /// grouping/search, so categories with no remaining items disappear
  /// (via [groupCatalogItems] omitting empty groups). Null = full catalog.
  final Set<String>? onlyIds;

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
    final l10n = AppLocalizations.of(context);
    return catalogAsync.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error:   (e, _) => Center(child: Text(l10n.failedToLoadCatalog('$e'))),
      data:    (items) => _buildBrowser(context, items),
    );
  }

  Widget _buildBrowser(BuildContext context, List<CatalogItem> items) {
    // Restrict the item universe before search/grouping when a filter is set
    // (e.g. the Changes tab showing only staged item ids). An empty set yields
    // no groups, which renders the existing generic "no items match" state.
    final onlyIds = widget.onlyIds;
    if (onlyIds != null) {
      items = items.where((i) => onlyIds.contains(i.id)).toList();
    }
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final lang = gameLangByCode(ref.watch(localeProvider));
    String nameOf(CatalogItem i) => displayNameForItem(i, locCatalog, lang);

    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;

    // Filter — match the raw class id, the derived name, and the localized name.
    final filtered = searching
        ? items.where((i) => i.id.toLowerCase().contains(query) ||
                             i.displayName.toLowerCase().contains(query) ||
                             nameOf(i).toLowerCase().contains(query)).toList()
        : items;

    final groups = groupCatalogItems(filtered);

    // Resolve selected category
    var selectedCat = _selectedCategory;
    if (groups.every((g) => g.category != selectedCat)) {
      selectedCat = groups.isEmpty ? null : groups.first.category;
    }

    final shownItems = sortByLocalizedName(
        searching
            ? filtered
            : (groups.where((g) => g.category == selectedCat).firstOrNull?.items ??
                const []),
        nameOf);

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              labelText: l10n.searchItems,
              prefixIcon: const Icon(Icons.search),
              isDense: true,
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          child: groups.isEmpty
              ? Center(child: Text(l10n.noItemsMatch))
              : Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!searching) SizedBox(
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
                                icon:     iconForItemCategory(g.category),
                                label:    l10n.categoryWithCount(
                                    g.category.localizedLabel(l10n),
                                    g.items.length),
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
                            title: Text(nameOf(item), maxLines: 1, overflow: TextOverflow.ellipsis),
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
