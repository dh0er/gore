import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

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
  });

  /// Item asset paths to exclude from the picker — the complete set of
  /// MainContainer items (addItem rejects paths already there). Sourced from
  /// the uncapped MainContainer path list, so it is correct even when the
  /// inventory list is truncated.
  final Set<String> excludePaths;
  final Future<ItemCatalog>? catalogOverride;

  @override
  ConsumerState<AddInventoryItemDialog> createState() =>
      _AddInventoryItemDialogState();
}

typedef _CatalogGroup = ({ItemCategory category, List<ItemCatalogEntry> entries});

class _AddInventoryItemDialogState
    extends ConsumerState<AddInventoryItemDialog> {
  // The core rejects addItem counts above i32::MAX (saving would fail with an
  // invalid-request error), so the dialog mirrors that upper bound.
  static const int _maxCount = 2147483647; // i32::MAX

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
    return localizedGameName(catalog, lang, id) ?? itemDisplayNameFromId(id);
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
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};
    return AlertDialog(
      title: Text(l10n.addItemDialogTitle),
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
            final available = catalog.entries
                .where((e) => !widget.excludePaths.contains(e.path))
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
                    e.path.toLowerCase().contains(query) ||
                    _displayName(locCatalog, lang, e.id)
                        .toLowerCase()
                        .contains(query);
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
                        _selectedCategory = itemCategoryFromId(_selected!.id);
                      }
                    } else if (_selected != null &&
                        !(_selected!.id.toLowerCase().contains(q) ||
                            _selected!.path.toLowerCase().contains(q) ||
                            _displayName(locCatalog, lang, _selected!.id)
                                .toLowerCase()
                                .contains(q))) {
                      // A search that no longer matches the selection drops it.
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
                          _displayName(locCatalog, lang, _selected!.id),
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
                                          label: l10n.categoryWithCount(
                                              localizedItemCategoryLabel(
                                                  l10n, g.category),
                                              g.entries.length),
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
                                  ? Center(child: Text(l10n.noItemsMatch))
                                  : ListView.builder(
                                      itemCount: shown.length,
                                      itemBuilder: (context, index) =>
                                          _entryTile(
                                        theme,
                                        shown[index],
                                        locCatalog,
                                        lang,
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
  ) {
    final isSelected = _selected == entry;
    return ListTile(
      dense: true,
      selected: isSelected,
      selectedTileColor: theme.colorScheme.primaryContainer,
      leading: Icon(iconForItemCategory(itemCategoryFromId(entry.id))),
      title: Text(
        _displayName(catalog, lang, entry.id),
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
