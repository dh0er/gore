import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/ui/inventory_item_visual.dart';
import 'package:goresave/features/editor/ui/item_stats_tooltip.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

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
class AddInventoryItemDialog extends ConsumerStatefulWidget {
  const AddInventoryItemDialog({
    super.key,
    required this.excludePaths,
    this.catalogOverride,
    this.warningText,
  });

  /// Item asset paths to exclude from the picker — the complete set of
  /// MainContainer items (addItem rejects paths already there). Sourced from
  /// the uncapped MainContainer path list, so it is correct even when the
  /// inventory list is truncated.
  final Set<String> excludePaths;
  final Future<ItemCatalog>? catalogOverride;

  /// Optional context-specific warning shown before the picker. Trader stock
  /// uses this to explain that a runtime maintenance pass may remove the new
  /// line; ordinary inventory adds leave it null.
  final String? warningText;

  @override
  ConsumerState<AddInventoryItemDialog> createState() =>
      _AddInventoryItemDialogState();
}

typedef _CatalogGroup = ({
  ItemCategory category,
  List<ItemCatalogEntry> entries,
});

class _AddInventoryItemDialogState
    extends ConsumerState<AddInventoryItemDialog> {
  // The core rejects addItem counts above i32::MAX (saving would fail with an
  // invalid-request error), so the dialog mirrors that upper bound.
  static const int _maxCount = 2147483647; // i32::MAX

  String _query = '';
  ItemCategory? _selectedCategory;
  ItemCatalogEntry? _selected;
  final TextEditingController _searchController = TextEditingController();
  final TextEditingController _countController = TextEditingController(
    text: '1',
  );
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
    final l10n = AppLocalizations.of(context);
    final parsed = int.tryParse(value.trim());
    setState(() {
      if (parsed == null || parsed < 1) {
        _countError = l10n.countMustBeAtLeast1;
      } else if (parsed > _maxCount) {
        _countError = l10n.countMustBeAtMost(_maxCount);
      } else {
        _countError = null;
      }
    });
  }

  /// Localized game name for [id] when the loc_catalog has it; falls back to the
  /// derived id-only name (legal posture preserved when no catalog is present).
  String _displayName(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
    String id,
  ) {
    return localizedGameName(catalog, lang, id) ??
        itemDisplayNameFromId(
          id,
          fallback: AppLocalizations.of(context).fallbackItem,
        );
  }

  bool get _canAdd {
    if (_selected == null) return false;
    final parsed = int.tryParse(_countController.text.trim());
    return parsed != null && parsed >= 1 && parsed <= _maxCount;
  }

  void _confirm() {
    final entry = _selected;
    if (entry == null) return;
    final parsed = int.tryParse(_countController.text.trim());
    if (parsed == null || parsed < 1 || parsed > _maxCount) return;
    Navigator.of(
      context,
    ).pop(InventoryItemAdd(path: entry.path, count: parsed));
  }

  List<_CatalogGroup> _group(
    List<ItemCatalogEntry> entries,
    ItemStatsCatalog? stats,
  ) {
    final byCategory = <ItemCategory, List<ItemCatalogEntry>>{};
    for (final entry in entries) {
      byCategory
          .putIfAbsent(itemCategoryFor(entry.id, stats: stats), () => [])
          .add(entry);
    }
    return [
      for (final cat in ItemCategory.values)
        if (byCategory.containsKey(cat))
          (category: cat, entries: byCategory[cat]!),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);
    // File the catalog by the game's own inventory tabs, so the dialog reads
    // like the inventory it adds to.
    final itemStats = ref.watch(itemStatsCatalogProvider).value;
    // The dialog can be opened before the catalog is there, and then groups by
    // the id prefix. When it arrives, items move — an ore nugget goes from
    // Miscellaneous to Materials — while the remembered tab still exists, so
    // the selection quietly vanished from the list it was picked in.
    ref.listen(itemStatsCatalogProvider, (previous, next) {
      final stats = next.value;
      final selected = _selected;
      if (stats == null || selected == null || previous?.value != null) return;
      final moved = itemCategoryFor(selected.id, stats: stats);
      if (moved != _selectedCategory) {
        setState(() => _selectedCategory = moved);
      }
    });
    // The game's own "All" filter is not a category — it collects everything —
    // so it is deliberately absent here.
    final filtersById = <ItemCategory, InventoryFilter>{
      for (final filter in itemStats?.filters ?? const <InventoryFilter>[])
        ?itemCategoryFromFilterId(filter.id): filter,
    };
    String categoryLabel(ItemCategory category) {
      final key = filtersById[category]?.nameKey ?? '';
      final fromGame = key.isEmpty
          ? null
          : resolveGameText(locCatalog, key, lang);
      return fromGame ?? localizedItemCategoryLabel(l10n, category);
    }

    return AlertDialog(
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l10n.addItemDialogTitle),
          if (widget.warningText != null) ...[
            const SizedBox(height: 10),
            DecoratedBox(
              decoration: BoxDecoration(
                color: theme.colorScheme.errorContainer,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Padding(
                padding: const EdgeInsets.all(10),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(
                      Icons.warning_amber_outlined,
                      size: 18,
                      color: theme.colorScheme.onErrorContainer,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        widget.warningText!,
                        key: const ValueKey('add-item-warning'),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onErrorContainer,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
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
                child: Text(l10n.failedToLoadCatalog('${snapshot.error}')),
              );
            }
            final catalog = snapshot.data!;
            final available =
                catalog.entries
                    .where((e) => !widget.excludePaths.contains(e.path))
                    .toList()
                  // Order by the localized name shown in the rows, not the raw id,
                  // so both the per-category list and the flat search read A→Z the
                  // way the user sees them. _group preserves this encounter order.
                  ..sort(
                    (a, b) => _displayName(locCatalog, lang, a.id)
                        .toLowerCase()
                        .compareTo(
                          _displayName(locCatalog, lang, b.id).toLowerCase(),
                        ),
                  );
            final groups = _group(available, itemStats);

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
                    e.path.toLowerCase().contains(query) ||
                    _displayName(
                      locCatalog,
                      lang,
                      e.id,
                    ).toLowerCase().contains(query);
              }).toList();
            } else {
              shown =
                  groups
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
                  decoration: InputDecoration(
                    labelText: l10n.searchItems,
                    prefixIcon: const Icon(Icons.search),
                    isDense: true,
                  ),
                  onChanged: (v) => setState(() {
                    _query = v;
                    final q = v.trim().toLowerCase();
                    if (q.isEmpty) {
                      // Reverting to category browsing: reveal the selected
                      // item's category so the selection stays visible instead
                      // of being silently dropped.
                      if (_selected != null) {
                        // The SAME classifier the grouping uses. The game's own
                        // tag overrides the id prefix — ore nuggets are filed
                        // under Materials, not Miscellaneous — so reading the
                        // category off the id alone opened the wrong tab and
                        // hid the very row it was meant to reveal.
                        _selectedCategory = itemCategoryFor(
                          _selected!.id,
                          stats: itemStats,
                        );
                      }
                    } else if (_selected != null &&
                        !(_selected!.id.toLowerCase().contains(q) ||
                            _selected!.path.toLowerCase().contains(q) ||
                            _displayName(
                              locCatalog,
                              lang,
                              _selected!.id,
                            ).toLowerCase().contains(q))) {
                      // A search that no longer matches the selection drops it.
                      _selected = null;
                    }
                  }),
                ),
                const SizedBox(height: 8),
                if (_selected != null) ...[
                  Row(
                    children: [
                      InventoryItemVisual(
                        key: ValueKey(('selected-item-image', _selected!.id)),
                        itemId: _selected!.id,
                        itemPath: _selected!.path,
                        fallbackIcon: iconForItemCategory(
                          itemCategoryFor(_selected!.id, stats: itemStats),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              _displayName(locCatalog, lang, _selected!.id),
                              style: theme.textTheme.bodyMedium,
                              overflow: TextOverflow.ellipsis,
                            ),
                            if (showObjectIds)
                              Text(
                                _selected!.id,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: theme.colorScheme.onSurfaceVariant,
                                  fontFamily: uiAwareMonospaceFontFamily(
                                    context,
                                  ),
                                ),
                              ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 8),
                      SizedBox(
                        width: 100,
                        child: TextField(
                          controller: _countController,
                          decoration: InputDecoration(
                            labelText: l10n.count,
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
                      ? Center(child: Text(l10n.noItemsAvailableToAdd))
                      : Row(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            SizedBox(
                              width: 240,
                              child: DecoratedBox(
                                decoration: BoxDecoration(
                                  color: theme.colorScheme.surfaceContainerLow,
                                  borderRadius: BorderRadius.circular(12),
                                ),
                                child: SingleChildScrollView(
                                  padding: const EdgeInsets.symmetric(
                                    vertical: 6,
                                  ),
                                  child: Column(
                                    children: [
                                      for (final g in groups)
                                        SidebarTile(
                                          icon: iconForItemCategory(g.category),
                                          gameIcon:
                                              filtersById[g.category]?.icon ??
                                              gameIconForItemCategory(
                                                g.category,
                                              ),
                                          label: l10n.categoryWithCount(
                                            categoryLabel(g.category),
                                            g.entries.length,
                                          ),
                                          selected:
                                              !searching &&
                                              g.category == selectedCat,
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
                                  ? Center(child: Text(l10n.noItemsMatch))
                                  : ListView.builder(
                                      itemCount: shown.length,
                                      itemBuilder: (context, index) =>
                                          _entryTile(
                                            theme,
                                            shown[index],
                                            locCatalog,
                                            lang,
                                            showObjectIds,
                                            itemStats,
                                          ),
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
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _canAdd ? _confirm : null,
          child: Text(l10n.add),
        ),
      ],
    );
  }

  Widget _entryTile(
    ThemeData theme,
    ItemCatalogEntry entry,
    Map<String, Map<String, String>> catalog,
    GameLang lang,
    bool showObjectIds,
    ItemStatsCatalog? stats,
  ) {
    final isSelected = _selected == entry;
    // Same hover block as the inventory, so the item can be judged before it is
    // added rather than after.
    return ItemStatsTooltip(
      itemId: entry.id,
      title: _displayName(catalog, lang, entry.id),
      // The tile is tappable and brings its own hover colour; a second tint on
      // top of it would only muddy the selected row.
      highlightOnHover: false,
      child: ListTile(
        dense: true,
        selected: isSelected,
        selectedTileColor: theme.colorScheme.primaryContainer,
        leading: InventoryItemVisual(
          key: ValueKey(('catalog-item-image', entry.id)),
          itemId: entry.id,
          itemPath: entry.path,
          fallbackIcon: iconForItemCategory(
            itemCategoryFor(entry.id, stats: stats),
          ),
        ),
        title: Text(
          _displayName(catalog, lang, entry.id),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: showObjectIds
            ? Text(entry.id, maxLines: 1, overflow: TextOverflow.ellipsis)
            : null,
        onTap: () => setState(() {
          _selected = isSelected ? null : entry;
        }),
      ),
    );
  }
}
