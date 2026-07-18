import 'package:flutter/material.dart';

import '../catalog/domain/field_schema.dart';
import '../l10n/app_localizations.dart';
import 'revision3_item_catalog.dart';

/// Managed, read-only browser for the bundled base-game item reference.
///
/// This view deliberately exposes no edit, create, save, build, or deploy
/// action. A later semantic R3 transaction can build on the catalog without
/// turning this presentation model into project state.
class Revision3ItemsView extends StatefulWidget {
  const Revision3ItemsView({
    this.load = loadRevision3BundledItemCatalog,
    super.key,
  });

  final Revision3ItemCatalogLoader load;

  @override
  State<Revision3ItemsView> createState() => _Revision3ItemsViewState();
}

class _Revision3ItemsViewState extends State<Revision3ItemsView> {
  late Future<Revision3ItemCatalog> _catalog;
  final TextEditingController _search = TextEditingController();
  String _query = '';
  Revision3ItemCategory? _category;
  String? _selectedId;
  bool _compactDetailVisible = false;

  @override
  void initState() {
    super.initState();
    _catalog = widget.load();
  }

  @override
  void didUpdateWidget(covariant Revision3ItemsView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.load != widget.load) {
      _resetAndReload();
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  void _resetAndReload() {
    _search.clear();
    _query = '';
    _category = null;
    _selectedId = null;
    _compactDetailVisible = false;
    _catalog = widget.load();
  }

  void _retry() => setState(_resetAndReload);

  void _changeQuery(String value) => setState(() {
    _query = value;
    _compactDetailVisible = false;
  });

  void _clearQuery() {
    _search.clear();
    _changeQuery('');
  }

  void _selectCategory(Revision3ItemCategory? category) => setState(() {
    _category = category;
    _compactDetailVisible = false;
  });

  void _selectItem(Revision3ItemCatalogEntry item) => setState(() {
    _selectedId = item.id;
    _compactDetailVisible = true;
  });

  @override
  Widget build(BuildContext context) => FutureBuilder<Revision3ItemCatalog>(
    future: _catalog,
    builder: (context, snapshot) {
      if (snapshot.connectionState != ConnectionState.done) {
        return const Center(
          child: CircularProgressIndicator(key: Key('revision3-items-loading')),
        );
      }
      if (snapshot.hasError) {
        return _ItemCatalogLoadError(
          error: snapshot.error.toString(),
          onRetry: _retry,
        );
      }
      return _catalogBody(context, snapshot.requireData);
    },
  );

  Widget _catalogBody(BuildContext context, Revision3ItemCatalog catalog) {
    final foldedQuery = _query.trim().toLowerCase();
    final filtered = catalog.items
        .where((item) {
          if (_category != null && item.category != _category) {
            return false;
          }
          return foldedQuery.isEmpty ||
              item.id.toLowerCase().contains(foldedQuery) ||
              item.displayName.toLowerCase().contains(foldedQuery);
        })
        .toList(growable: false);
    final counts = <Revision3ItemCategory, int>{};
    for (final item in catalog.items) {
      counts.update(item.category, (count) => count + 1, ifAbsent: () => 1);
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 760;
        Revision3ItemCatalogEntry? selected;
        for (final item in filtered) {
          if (item.id == _selectedId) {
            selected = item;
            break;
          }
        }
        if (wide && selected == null && filtered.isNotEmpty) {
          selected = filtered.first;
        }

        final browser = _ItemBrowser(
          items: filtered,
          totalCount: catalog.items.length,
          counts: counts,
          selectedId: selected?.id,
          category: _category,
          searchController: _search,
          onQueryChanged: _changeQuery,
          onClearQuery: _clearQuery,
          onCategoryChanged: _selectCategory,
          onSelected: _selectItem,
        );
        if (!wide) {
          if (_compactDetailVisible && selected != null) {
            return _ItemDetails(
              item: selected,
              compact: true,
              onBack: () => setState(() => _compactDetailVisible = false),
            );
          }
          return browser;
        }

        final browserWidth = (constraints.maxWidth * 0.38).clamp(330.0, 440.0);
        return Row(
          key: const Key('revision3-items-wide-layout'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(width: browserWidth, child: browser),
            const VerticalDivider(width: 1),
            Expanded(
              child: selected == null
                  ? _EmptySelection(queryActive: filtered.isEmpty)
                  : _ItemDetails(item: selected, compact: false),
            ),
          ],
        );
      },
    );
  }
}

class _ItemCatalogLoadError extends StatelessWidget {
  const _ItemCatalogLoadError({required this.error, required this.onRetry});

  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final minimumHeight = (constraints.maxHeight - 32).clamp(
          0.0,
          double.infinity,
        );
        return SingleChildScrollView(
          key: const Key('revision3-items-load-error-scroll'),
          padding: const EdgeInsets.all(16),
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: minimumHeight),
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 560),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.error_outline,
                      size: 36,
                      color: Theme.of(context).colorScheme.error,
                    ),
                    const SizedBox(height: 12),
                    Text(
                      l10n.failedToLoadCatalog(error),
                      key: const Key('revision3-items-load-error'),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 16),
                    OutlinedButton.icon(
                      key: const Key('revision3-items-retry'),
                      onPressed: onRetry,
                      icon: const Icon(Icons.refresh),
                      label: Text(l10n.managedDashboardRetry),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _ItemBrowser extends StatelessWidget {
  const _ItemBrowser({
    required this.items,
    required this.totalCount,
    required this.counts,
    required this.selectedId,
    required this.category,
    required this.searchController,
    required this.onQueryChanged,
    required this.onClearQuery,
    required this.onCategoryChanged,
    required this.onSelected,
  });

  final List<Revision3ItemCatalogEntry> items;
  final int totalCount;
  final Map<Revision3ItemCategory, int> counts;
  final String? selectedId;
  final Revision3ItemCategory? category;
  final TextEditingController searchController;
  final ValueChanged<String> onQueryChanged;
  final VoidCallback onClearQuery;
  final ValueChanged<Revision3ItemCategory?> onCategoryChanged;
  final ValueChanged<Revision3ItemCatalogEntry> onSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      key: const Key('revision3-items-browser'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: TextField(
            key: const Key('revision3-items-search'),
            controller: searchController,
            onChanged: onQueryChanged,
            textInputAction: TextInputAction.search,
            decoration: InputDecoration(
              labelText: l10n.searchItems,
              prefixIcon: const Icon(Icons.search),
              suffixIcon: searchController.text.isEmpty
                  ? null
                  : IconButton(
                      key: const Key('revision3-items-clear-search'),
                      tooltip: l10n.clearAll,
                      onPressed: onClearQuery,
                      icon: const Icon(Icons.clear),
                    ),
              border: const OutlineInputBorder(),
            ),
          ),
        ),
        SingleChildScrollView(
          key: const Key('revision3-items-category-scroll'),
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.fromLTRB(12, 4, 12, 8),
          child: Row(
            children: [
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: ChoiceChip(
                  key: const Key('revision3-items-category-all'),
                  selected: category == null,
                  label: Text(
                    l10n.categoryWithCount(l10n.changesAll, totalCount),
                  ),
                  onSelected: (_) => onCategoryChanged(null),
                ),
              ),
              for (final itemCategory in Revision3ItemCategory.values)
                if (counts[itemCategory] case final count?)
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 4),
                    child: ChoiceChip(
                      key: ValueKey(
                        'revision3-items-category-${itemCategory.name}',
                      ),
                      selected: category == itemCategory,
                      label: Text(
                        l10n.categoryWithCount(
                          _categoryLabel(l10n, itemCategory),
                          count,
                        ),
                      ),
                      onSelected: (_) => onCategoryChanged(itemCategory),
                    ),
                  ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: items.isEmpty
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(
                      l10n.noItemsMatch,
                      key: const Key('revision3-items-empty'),
                      textAlign: TextAlign.center,
                    ),
                  ),
                )
              : ListView.builder(
                  key: const Key('revision3-items-results'),
                  itemCount: items.length,
                  itemBuilder: (context, index) {
                    final item = items[index];
                    final selected = item.id == selectedId;
                    return ListTile(
                      key: ValueKey('revision3-items-result-${item.id}'),
                      selected: selected,
                      leading: const Icon(Icons.inventory_2_outlined),
                      title: Text(
                        item.displayName,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(
                        item.id,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => onSelected(item),
                    );
                  },
                ),
        ),
      ],
    );
  }
}

class _ItemDetails extends StatelessWidget {
  const _ItemDetails({required this.item, required this.compact, this.onBack});

  final Revision3ItemCatalogEntry item;
  final bool compact;
  final VoidCallback? onBack;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final category = item.category;
    return ListView(
      key: ValueKey('revision3-items-details-${item.id}'),
      padding: const EdgeInsets.fromLTRB(20, 16, 20, 32),
      children: [
        if (compact)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: const Key('revision3-items-back'),
              onPressed: onBack,
              icon: const Icon(Icons.arrow_back),
              label: Text(l10n.tabItems),
            ),
          ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Chip(
              avatar: const Icon(Icons.sports_esports_outlined, size: 18),
              label: Text(l10n.managedContentScopeBaseGameLabel),
            ),
            Chip(
              avatar: const Icon(Icons.visibility_outlined, size: 18),
              label: Text(l10n.managedBaseGameBrowserInspectOnlyBadge),
            ),
            Chip(
              avatar: const Icon(Icons.inventory_2_outlined, size: 18),
              label: Text(l10n.managedItemsBundledReferenceBadge),
            ),
            Chip(label: Text(_categoryLabel(l10n, category))),
          ],
        ),
        const SizedBox(height: 14),
        SelectableText(
          item.displayName,
          key: const Key('revision3-items-detail-name'),
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 8),
        Text(
          l10n.managedStoryWorkbenchTechnicalIdLabel,
          style: Theme.of(context).textTheme.labelMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        SelectableText(
          item.id,
          key: const Key('revision3-items-detail-id'),
          style: Theme.of(
            context,
          ).textTheme.bodyLarge?.copyWith(fontFamily: 'monospace'),
        ),
        const SizedBox(height: 16),
        Card.filled(
          key: const Key('revision3-items-bundled-boundary'),
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(Icons.info_outline, size: 20),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(l10n.managedItemsBundledReferenceBoundary),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 24),
        Text(
          l10n.categoryWithCount(l10n.sectionItemValues, item.fields.length),
          key: const Key('revision3-items-field-heading'),
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        if (item.fields.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 28),
            child: Text(
              l10n.managedItemsNoKnownFields,
              key: const Key('revision3-items-no-known-fields'),
              textAlign: TextAlign.center,
            ),
          )
        else
          for (final field in item.fields) _FieldCard(item: item, field: field),
      ],
    );
  }
}

class _FieldCard extends StatelessWidget {
  const _FieldCard({required this.item, required this.field});

  final Revision3ItemCatalogEntry item;
  final FieldSchema field;

  @override
  Widget build(BuildContext context) {
    final defaultValue = _displayDefault(field);
    return Card(
      key: ValueKey('revision3-items-field-${item.id}-${field.name}'),
      margin: const EdgeInsets.symmetric(vertical: 5),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SelectableText(
              field.name,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontFamily: 'monospace'),
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _FactChip(
                  icon: Icons.data_object,
                  text: _fieldType(field.type),
                ),
                if (defaultValue != null)
                  _FactChip(
                    key: ValueKey(
                      'revision3-items-field-${item.id}-${field.name}-default',
                    ),
                    icon: Icons.subdirectory_arrow_right,
                    text: '= $defaultValue',
                  ),
                if (field.minValue != null)
                  _FactChip(
                    icon: Icons.keyboard_arrow_up,
                    text: '\u2265 ${field.minValue}',
                  ),
                if (field.maxValue != null)
                  _FactChip(
                    icon: Icons.keyboard_arrow_down,
                    text: '\u2264 ${field.maxValue}',
                  ),
              ],
            ),
            if (field.type == FieldType.enum_ &&
                field.enumValues.isNotEmpty) ...[
              const SizedBox(height: 10),
              Text(
                field.enumValues.join(' \u00b7 '),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _FactChip extends StatelessWidget {
  const _FactChip({required this.icon, required this.text, super.key});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => Chip(
    avatar: Icon(icon, size: 16),
    label: Text(text),
    visualDensity: VisualDensity.compact,
  );
}

class _EmptySelection extends StatelessWidget {
  const _EmptySelection({required this.queryActive});

  final bool queryActive;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Icon(
        queryActive ? Icons.search_off : Icons.inventory_2_outlined,
        size: 44,
        color: Theme.of(context).colorScheme.outline,
      ),
    ),
  );
}

String _categoryLabel(AppLocalizations l10n, Revision3ItemCategory category) =>
    switch (category) {
      Revision3ItemCategory.meleeWeapon => l10n.categoryMeleeWeapons,
      Revision3ItemCategory.rangedWeapon => l10n.categoryRangedWeapons,
      Revision3ItemCategory.ammunition => l10n.categoryAmmunition,
      Revision3ItemCategory.rune => l10n.categoryRunes,
      Revision3ItemCategory.scroll => l10n.categorySpellScrolls,
      Revision3ItemCategory.food => l10n.categoryFoodAndPotions,
      Revision3ItemCategory.misc => l10n.categoryMiscellaneous,
      Revision3ItemCategory.amulet => l10n.categoryAmulets,
      Revision3ItemCategory.ring => l10n.categoryRings,
      Revision3ItemCategory.trophy => l10n.categoryAnimalTrophies,
      Revision3ItemCategory.writing => l10n.categoryWritings,
      Revision3ItemCategory.mission => l10n.categoryMissionItems,
      Revision3ItemCategory.key => l10n.categoryKeys,
      Revision3ItemCategory.special => l10n.managedItemsCategorySpecial,
    };

String _fieldType(FieldType type) => switch (type) {
  FieldType.int_ => 'int',
  FieldType.float_ => 'float',
  FieldType.bool_ => 'bool',
  FieldType.string_ => 'string',
  FieldType.enum_ => 'enum',
};

String? _displayDefault(FieldSchema field) {
  final value = field.defaultValue;
  if (value == null) return null;
  if (field.type == FieldType.string_) return '"$value"';
  if (field.type != FieldType.enum_ || value is! int) return value.toString();

  var memberIndex = -1;
  if (field.enumBackingValues.isNotEmpty) {
    memberIndex = field.enumBackingValues.indexOf(value);
  } else if (value >= 0 && value < field.enumValues.length) {
    memberIndex = value;
  }
  return memberIndex < 0
      ? value.toString()
      : '${field.enumValues[memberIndex]} ($value)';
}
