import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/features/editor/ui/add_inventory_item_dialog.dart';
import 'package:goresave/features/editor/ui/pending_structural_row.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// The Handel (trade) sub-tab: what a merchant offers and how much ore he has
/// to buy with.
///
/// This is NOT his inventory. A merchant's shop lives in a global array keyed by
/// his unique name, and it carries two maps — the live stock and the baseline he
/// restocks toward. His ore sits inside the same map as an ordinary line,
/// because ore is the currency and what he holds is what he can pay with.
class TraderPanel extends ConsumerStatefulWidget {
  const TraderPanel({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.actor,
    required this.editable,
    required this.reloadKey,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final Actor actor;

  /// Same save-wide gate the other editing panes take
  /// (`privateEditable && privateTypedVerified && codecCompressReady`).
  final bool editable;

  /// Changes when the panel must re-read from disk: a different merchant, or the
  /// same one after a save re-inspected the file. Without the inspection in here
  /// a save would leave the panel showing the pre-save stock — the tab is kept
  /// alive across switches, so nothing else would ever trigger the reload.
  final Object reloadKey;

  @override
  ConsumerState<TraderPanel> createState() => _TraderPanelState();
}

class _TraderPanelState extends ConsumerState<TraderPanel> {
  TradersResult? _list;
  TraderDetail? _detail;
  String? _error;
  bool _loading = true;

  /// Guards against a slow reload landing after a newer one: only the newest
  /// epoch may write to the state.
  int _epoch = 0;

  /// Which of the two stock maps is on screen. They hold the same kind of data
  /// and are edited the same way, so showing both at once only invited the
  /// question which one the ore field belonged to.
  TraderStockMap _map = TraderStockMap.current;

  /// Which category the sidebar has selected. Null until the first build picks
  /// one, and reset whenever the selection no longer has any lines — switching
  /// maps can empty it.
  ItemCategory? _category;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant TraderPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _load();
  }

  Future<void> _load() async {
    final epoch = ++_epoch;
    setState(() {
      _loading = true;
      _error = null;
      _detail = null;
    });
    final list = await widget.notifier.loadTraders();
    if (!mounted || epoch != _epoch) return;
    if (list.error != null) {
      setState(() {
        _loading = false;
        _error = list.error;
        _list = null;
      });
      return;
    }
    final row = list.forUniqueName(widget.actor.uniqueName);
    if (row == null) {
      // Not a merchant. A clean empty state, not an error.
      setState(() {
        _loading = false;
        _list = list;
        _detail = null;
      });
      return;
    }
    final detail = await widget.notifier.loadTraderDetail(row.index);
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _loading = false;
      _list = list;
      _error = detail.error;
      _detail = detail.detail;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Rebuild when pending edits change so a reverted field drops its badge.
    ref.watch(editorProvider.select((s) => s.pendingEdits.length));

    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return _Message(
        icon: Icons.error_outline,
        title: l10n.tabTrade,
        body: _error!,
        onRetry: _load,
      );
    }
    final detail = _detail;
    if (detail == null) {
      return _Message(
        icon: Icons.storefront_outlined,
        title: l10n.tabTrade,
        body: l10n.traderNotAMerchant,
      );
    }

    final list = _list;
    // Per-difficulty stock is not modelled, and the edits reach only m_Items and
    // m_DefaultItems. A save that carries it would take an edit, report success,
    // and leave that other stock standing — so nothing here is editable then.
    final unsupported = detail.hasItemsByDifficulty;
    final canSet =
        widget.editable && !unsupported && (list?.canSetStock ?? false);
    final canAdd =
        widget.editable && !unsupported && (list?.canAddItem ?? false);
    final canRemove =
        widget.editable && !unsupported && (list?.canRemoveItem ?? false);

    // The live stock gets the ore its own card, because that number is the
    // merchant's purchasing power and not just another line. The restock
    // baseline has no such meaning, so there its ore stays an ordinary row.
    final showOreCard = _map == TraderStockMap.current;
    final removals = _pendingRemovals(_map);
    final rows = [
      for (final item in detail.stock(_map))
        if (!(showOreCard && item.isOre) && !removals.contains(item.path)) item,
    ];

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // First, because it qualifies every number below it — the ore as much
          // as the stock counts.
          if (unsupported) ...[
            _NoteCard(
              text: l10n.traderDifficultyStockUnsupported,
              tone: _NoteTone.warning,
            ),
            const SizedBox(height: 12),
          ],
          _NoteCard(text: l10n.traderPriceWarning),
          if (widget.editable &&
              !unsupported &&
              !(list?.canSetStock ?? false)) ...[
            const SizedBox(height: 12),
            Text(l10n.traderReadOnlyCore, style: theme.textTheme.bodySmall),
          ],
          const SizedBox(height: 16),
          Align(
            alignment: Alignment.centerLeft,
            child: SegmentedButton<TraderStockMap>(
              segments: [
                ButtonSegment(
                  value: TraderStockMap.current,
                  icon: const Icon(Icons.storefront_outlined),
                  label: Text(l10n.traderStockCurrent),
                ),
                ButtonSegment(
                  value: TraderStockMap.base,
                  icon: const Icon(Icons.inventory_outlined),
                  label: Text(l10n.traderStockBase),
                ),
              ],
              selected: {_map},
              onSelectionChanged: (selection) =>
                  setState(() => _map = selection.first),
            ),
          ),
          if (_map == TraderStockMap.base) ...[
            const SizedBox(height: 8),
            Text(l10n.traderStockBaseHint, style: theme.textTheme.bodySmall),
          ],
          if (showOreCard) ...[
            const SizedBox(height: 16),
            _OreCard(
              detail: detail,
              editable: canSet,
              canRemove: canRemove,
              removalPending: removals.contains(kTraderOrePath),
              onChanged: (value) => _queueSet(_map, kTraderOrePath, value),
              onRevert: () => _revert(_map, kTraderOrePath),
              onRemove: () => _queueRemove(_map, kTraderOrePath),
              pending: _pendingCountFor(_map, kTraderOrePath),
            ),
          ],
          const SizedBox(height: 16),
          Expanded(
            child: _StockSection(
              map: _map,
              items: rows,
              lineCount: detail.stock(_map).length,
              pendingAdds: _pendingAdds(_map),
              pendingRemovals: [
                for (final item in detail.stock(_map))
                  if (removals.contains(item.path)) item,
              ],
              canSet: canSet,
              canAdd: canAdd,
              canRemove: canRemove,
              selectedCategory: _category,
              onSelectCategory: (category) =>
                  setState(() => _category = category),
              pendingOf: _pendingCountFor,
              onChanged: _queueSet,
              onRevert: _revert,
              onRemove: _queueRemove,
              onRevertAdd: _revertAdd,
              onAdd: () => _addItem(_map, detail),
            ),
          ),
        ],
      ),
    );
  }

  int get _index => _detail!.summary.index;

  TraderStockEdit _edit(
    TraderEditKind kind,
    TraderStockMap map,
    String path, {
    int count = 0,
  }) => TraderStockEdit(
    kind: kind,
    index: _index,
    map: map,
    path: path,
    count: count,
  );

  /// The queued count for a line, or null when nothing is queued. Reads the
  /// notifier's pending map rather than local state so the badge survives a
  /// rebuild and matches what a save would actually send.
  int? _pendingCountFor(TraderStockMap map, String path) {
    final key = _edit(TraderEditKind.setStock, map, path).pendingKey;
    final pending = ref.read(editorProvider).pendingEdits[key];
    final value = pending?.edits.firstOrNull?['value'];
    if (value is Map && value['count'] is num) {
      return (value['count'] as num).toInt();
    }
    return null;
  }

  bool _isRemovalPending(TraderStockMap map, String path) =>
      _pendingRemovals(map).contains(path);

  /// Item paths queued for removal. They are taken OUT of the list and shown as
  /// a banner above it instead: a struck-through row still reads as something
  /// the save contains, and after the write it will not.
  Set<String> _pendingRemovals(TraderStockMap map) {
    final prefix = 'traders:$_index:${map.wire}:';
    final out = <String>{};
    ref.read(editorProvider).pendingEdits.forEach((key, pending) {
      if (!key.startsWith(prefix)) return;
      final edit = pending.edits.firstOrNull;
      if (edit?['path'] != 'private.traders.removeItem') return;
      final value = edit?['value'];
      if (value is Map && value['path'] is String) {
        out.add(value['path'] as String);
      }
    });
    return out;
  }

  /// Lines queued for insertion but not saved yet.
  ///
  /// A new line has no counterpart in the loaded stock, so it would otherwise be
  /// invisible until the next save — the inventory shows its queued additions
  /// the same way. Read out of the notifier rather than a local list so a tab
  /// switch (which keeps this panel alive but rebuilds it) cannot lose them.
  List<TraderItem> _pendingAdds(TraderStockMap map) {
    final prefix = 'traders:$_index:${map.wire}:';
    final out = <TraderItem>[];
    ref.read(editorProvider).pendingEdits.forEach((key, pending) {
      if (!key.startsWith(prefix)) return;
      final edit = pending.edits.firstOrNull;
      if (edit?['path'] != 'private.traders.addItem') return;
      final value = edit?['value'];
      if (value is! Map) return;
      final path = value['path'] as String? ?? '';
      if (path.isEmpty) return;
      out.add(
        TraderItem(
          path: path,
          id: path.split('.').last,
          count: (value['count'] as num?)?.toInt() ?? 0,
          unknownItem: false,
        ),
      );
    });
    out.sort((a, b) => a.id.compareTo(b.id));
    return out;
  }

  void _revertAdd(TraderStockMap map, String path) {
    widget.notifier.clearTraderStockEdit(
      _edit(TraderEditKind.addItem, map, path),
    );
    setState(() {});
  }

  void _queueSet(TraderStockMap map, String path, int count) {
    widget.notifier.setTraderStockEdit(
      _edit(TraderEditKind.setStock, map, path, count: count),
    );
    setState(() {});
  }

  void _revert(TraderStockMap map, String path) {
    widget.notifier.clearTraderStockEdit(_edit(TraderEditKind.setStock, map, path));
    setState(() {});
  }

  void _queueRemove(TraderStockMap map, String path) {
    final edit = _edit(TraderEditKind.removeItem, map, path);
    if (_isRemovalPending(map, path)) {
      widget.notifier.clearTraderStockEdit(edit);
    } else {
      // A removal supersedes a queued count change on the same line: sending
      // both would set a value and then delete the line it lives in.
      widget.notifier.clearTraderStockEdit(
        _edit(TraderEditKind.setStock, map, path),
      );
      widget.notifier.setTraderStockEdit(edit);
    }
    setState(() {});
  }

  Future<void> _addItem(TraderStockMap map, TraderDetail detail) async {
    final savePath = widget.notifier.selectedPath;
    final held = {for (final i in detail.stock(map)) i.path};
    final result = await showDialog<InventoryItemAdd>(
      context: context,
      // The core refuses a duplicate key, so never offer a line he already has.
      builder: (_) => AddInventoryItemDialog(excludePaths: held),
    );
    if (result == null) return;
    if (!mounted || widget.notifier.selectedPath != savePath) return;
    widget.notifier.setTraderStockEdit(
      _edit(TraderEditKind.addItem, map, result.path, count: result.count),
    );
    setState(() {});
  }
}

class _OreCard extends ConsumerWidget {
  const _OreCard({
    required this.detail,
    required this.editable,
    required this.canRemove,
    required this.removalPending,
    required this.onChanged,
    required this.onRevert,
    required this.onRemove,
    required this.pending,
  });

  final TraderDetail detail;
  final bool editable;

  /// Whether the ore line may be dropped entirely. A merchant without one is a
  /// state the game itself produces, so the card has to offer it.
  final bool canRemove;
  final bool removalPending;
  final void Function(int) onChanged;
  final VoidCallback onRevert;
  final VoidCallback onRemove;
  final int? pending;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final ore = detail.summary.ore;
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(Icons.savings_outlined, color: theme.colorScheme.primary),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(l10n.traderOre, style: theme.textTheme.titleMedium),
                  const SizedBox(height: 4),
                  Text(l10n.traderOreHint, style: theme.textTheme.bodySmall),
                ],
              ),
            ),
            const SizedBox(width: 12),
            if (ore == null)
              // No ore line at all is a real state and NOT the same as zero, so
              // say so instead of showing a 0 the save does not contain.
              Text(l10n.traderNoOre, style: theme.textTheme.bodyMedium)
            else ...[
              SizedBox(
                width: 140,
                child: _CountField(
                  value: ore,
                  pending: pending,
                  // While a removal is queued the number is on its way out;
                  // editing it would queue a count for a line about to go.
                  enabled: editable && !removalPending,
                  onChanged: onChanged,
                  onRevert: onRevert,
                ),
              ),
              if (canRemove)
                IconButton(
                  tooltip: l10n.traderRemoveItem,
                  icon: const Icon(Icons.delete_outline, size: 20),
                  onPressed: removalPending ? null : onRemove,
                ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Whether a note merely explains something or reports a limit that stops the
/// panel from doing what it otherwise would.
enum _NoteTone { info, warning }

class _NoteCard extends StatelessWidget {
  const _NoteCard({required this.text, this.tone = _NoteTone.info});

  final String text;
  final _NoteTone tone;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isWarning = tone == _NoteTone.warning;
    return Card(
      margin: EdgeInsets.zero,
      color: isWarning ? theme.colorScheme.errorContainer : null,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              isWarning ? Icons.warning_amber_outlined : Icons.info_outline,
              size: 18,
              color: isWarning
                  ? theme.colorScheme.onErrorContainer
                  : theme.colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                text,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: isWarning ? theme.colorScheme.onErrorContainer : null,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StockSection extends ConsumerWidget {
  const _StockSection({
    required this.map,
    required this.items,
    required this.lineCount,
    required this.pendingAdds,
    required this.pendingRemovals,
    required this.canSet,
    required this.canAdd,
    required this.canRemove,
    required this.selectedCategory,
    required this.onSelectCategory,
    required this.pendingOf,
    required this.onChanged,
    required this.onRevert,
    required this.onRemove,
    required this.onRevertAdd,
    required this.onAdd,
  });

  /// Below this width the sidebar would leave the list unusably narrow, so the
  /// categories collapse into one flat list instead. Same threshold the
  /// inventory browser uses.
  static const double _compactBelow = 600;

  final TraderStockMap map;

  /// The rows to draw: saved lines minus the ones queued for removal, and minus
  /// the ore when it has its own card.
  final List<TraderItem> items;

  /// How many lines the map holds on disk. The header states this rather than
  /// [items].length, which no longer counts the rows filtered out of the view.
  final int lineCount;
  final List<TraderItem> pendingAdds;
  final List<TraderItem> pendingRemovals;
  final bool canSet;
  final bool canAdd;
  final bool canRemove;
  final ItemCategory? selectedCategory;
  final void Function(ItemCategory) onSelectCategory;
  final int? Function(TraderStockMap, String) pendingOf;
  final void Function(TraderStockMap, String, int) onChanged;
  final void Function(TraderStockMap, String) onRevert;
  final void Function(TraderStockMap, String) onRemove;
  final void Function(TraderStockMap, String) onRevertAdd;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Sort by the name the user actually reads, the way the inventory does.
    // `.value` (not `.asData?.value`) so a background catalog refresh keeps the
    // previous order instead of briefly re-sorting by raw class id.
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    String nameOf(TraderItem item) =>
        localizedGameName(locCatalog, lang, item.id) ?? item.id;
    final groups = _grouped(items, displayNameOf: nameOf);
    // Hold the chosen category while it still has lines; otherwise fall back to
    // the first one so the list is never blank next to a populated sidebar.
    final selected = groups.any((g) => g.category == selectedCategory)
        ? selectedCategory
        : (groups.isEmpty ? null : groups.first.category);
    final shown = groups
            .where((g) => g.category == selected)
            .firstOrNull
            ?.items ??
        const <TraderItem>[];
    final nothingToShow =
        items.isEmpty && pendingAdds.isEmpty && pendingRemovals.isEmpty;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                l10n.traderStockLineCount(lineCount),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.outline,
                ),
              ),
            ),
            if (canAdd)
              OutlinedButton.icon(
                icon: const Icon(Icons.add),
                label: Text(l10n.traderAddItem),
                onPressed: onAdd,
              ),
          ],
        ),
        // Queued changes sit ABOVE the list: they are what the next save will
        // do, while the list below is what the save holds right now.
        for (final item in pendingAdds) ...[
          const SizedBox(height: 8),
          _PendingLineRow(
            item: item,
            tone: PendingTone.add,
            onCancel: () => onRevertAdd(map, item.path),
          ),
        ],
        for (final item in pendingRemovals) ...[
          const SizedBox(height: 8),
          _PendingLineRow(
            item: item,
            tone: PendingTone.remove,
            onCancel: () => onRemove(map, item.path),
          ),
        ],
        const SizedBox(height: 8),
        if (nothingToShow)
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              l10n.traderEmptyStock,
              style: theme.textTheme.bodyMedium,
            ),
          )
        else
          Expanded(
            child: LayoutBuilder(
              builder: (context, constraints) {
                final compact = constraints.maxWidth < _compactBelow;
                final rows = compact ? items : shown;
                return Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!compact) ...[
                      SizedBox(
                        width: 200,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerLow,
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: SingleChildScrollView(
                            padding: const EdgeInsets.symmetric(vertical: 6),
                            child: Column(
                              children: [
                                for (final group in groups)
                                  SidebarTile(
                                    icon: iconForItemCategory(group.category),
                                    label: l10n.categoryWithCount(
                                      localizedItemCategoryLabel(
                                        l10n,
                                        group.category,
                                      ),
                                      group.items.length,
                                    ),
                                    selected: group.category == selected,
                                    onTap: () => onSelectCategory(
                                      group.category,
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 16),
                    ],
                    Expanded(
                      child: ListView.builder(
                        padding: const EdgeInsets.symmetric(vertical: 4),
                        itemCount: rows.length,
                        itemBuilder: (context, index) => _StockRow(
                          key: ValueKey(rows[index].path),
                          item: rows[index],
                          map: map,
                          canSet: canSet,
                          canRemove: canRemove,
                          pending: pendingOf(map, rows[index].path),
                          onChanged: (v) =>
                              onChanged(map, rows[index].path, v),
                          onRevert: () => onRevert(map, rows[index].path),
                          onRemove: () => onRemove(map, rows[index].path),
                        ),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
      ],
    );
  }
}

/// One category's lines, in [ItemCategory] declaration order.
class _StockGroup {
  const _StockGroup({required this.category, required this.items});

  final ItemCategory category;
  final List<TraderItem> items;
}

/// Group a stock map the way the inventory groups its own items — same
/// classifier, so a sword lands under Melee weapons in both places, and the same
/// sort: case-insensitively by the localized name the user reads, with the class
/// id as a stable tiebreak.
List<_StockGroup> _grouped(
  List<TraderItem> items, {
  required String Function(TraderItem item) displayNameOf,
}) {
  final byCategory = <ItemCategory, List<TraderItem>>{};
  for (final item in items) {
    byCategory.putIfAbsent(itemCategoryFromId(item.id), () => []).add(item);
  }
  int compare(TraderItem a, TraderItem b) {
    final byName = displayNameOf(
      a,
    ).toLowerCase().compareTo(displayNameOf(b).toLowerCase());
    return byName != 0 ? byName : a.id.compareTo(b.id);
  }

  return [
    for (final category in ItemCategory.values)
      if (byCategory.containsKey(category))
        _StockGroup(
          category: category,
          items: byCategory[category]!..sort(compare),
        ),
  ];
}

/// A queued change, shown above the list rather than inside it. An insertion
/// has no row yet, and a removal's row is about to stop existing — drawing
/// either among the saved lines would claim a state the save does not have.
class _PendingLineRow extends ConsumerWidget {
  const _PendingLineRow({
    required this.item,
    required this.tone,
    required this.onCancel,
  });

  final TraderItem item;
  final PendingTone tone;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);
    final isAdd = tone == PendingTone.add;
    return PendingStructuralRow(
      tone: tone,
      icon: isAdd ? Icons.add_circle_outline : Icons.delete_outline,
      title: localizedGameName(locCatalog, lang, item.id) ?? item.id,
      subtitle: isAdd
          ? l10n.pendingAddSubtitle(item.count)
          : l10n.pendingRemovalSubtitle,
      technicalId: showObjectIds ? item.path : null,
      cancelTooltip: isAdd ? l10n.cancelPendingAdd : l10n.cancelPendingRemoval,
      onCancel: onCancel,
    );
  }
}

class _StockRow extends ConsumerWidget {
  const _StockRow({
    super.key,
    required this.item,
    required this.map,
    required this.canSet,
    required this.canRemove,
    required this.pending,
    required this.onChanged,
    required this.onRevert,
    required this.onRemove,
  });

  final TraderItem item;
  final TraderStockMap map;
  final bool canSet;
  final bool canRemove;
  final int? pending;
  final void Function(int) onChanged;
  final VoidCallback onRevert;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final lang = ref.watch(currentGameLangProvider);
    // `.value` (not `.asData?.value`) so a background refresh keeps the previous
    // catalog instead of briefly dropping every row back to its raw class id.
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final label = localizedGameName(locCatalog, lang, item.id) ?? item.id;
    final showObjectIds = ref.watch(showObjectIdsProvider);
    // The id repeats the title whenever no localized name exists, so drop it
    // then rather than printing the same string twice.
    final id = showObjectIds && label != item.id ? item.id : null;
    final subtitle = [
      ?id,
      if (item.unknownItem) l10n.traderUnknownItem,
    ].join(' · ');

    return ListTile(
      dense: true,
      leading: item.isOre
          ? Icon(Icons.savings_outlined, color: theme.colorScheme.primary)
          : const Icon(Icons.inventory_2_outlined),
      title: Text(label),
      subtitle: subtitle.isEmpty
          ? null
          : Text(subtitle, style: theme.textTheme.bodySmall),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 130,
            child: _CountField(
              value: item.count,
              pending: pending,
              // An unknown class is shown but never edited: we cannot vouch for
              // what the game does with a line it does not recognise.
              enabled: canSet && !item.unknownItem,
              onChanged: onChanged,
              onRevert: onRevert,
            ),
          ),
          if (canRemove)
            IconButton(
              tooltip: l10n.traderRemoveItem,
              icon: const Icon(Icons.delete_outline, size: 20),
              onPressed: onRemove,
            ),
        ],
      ),
    );
  }
}

/// A count field that shows the saved value until the user changes it, then
/// shows the queued value with a revert affordance.
class _CountField extends StatefulWidget {
  const _CountField({
    required this.value,
    required this.pending,
    required this.enabled,
    required this.onChanged,
    required this.onRevert,
  });

  final int value;
  final int? pending;
  final bool enabled;
  final void Function(int) onChanged;
  final VoidCallback onRevert;

  @override
  State<_CountField> createState() => _CountFieldState();
}

class _CountFieldState extends State<_CountField> {
  late final TextEditingController _controller = TextEditingController(
    text: '${widget.pending ?? widget.value}',
  );

  @override
  void didUpdateWidget(covariant _CountField oldWidget) {
    super.didUpdateWidget(oldWidget);
    final shown = widget.pending ?? widget.value;
    final inputsChanged =
        oldWidget.pending != widget.pending || oldWidget.value != widget.value;
    // Only overwrite when the field is not the thing that produced the change,
    // otherwise typing fights the controller.
    if (inputsChanged && '$shown' != _controller.text) {
      _controller.text = '$shown';
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// The core stores a count as an `i32`, so anything larger is refused at save
  /// time. The add-item dialog already caps at the same value.
  static const int _maxCount = 2147483647; // i32::MAX

  void _submit(String raw) {
    final parsed = int.tryParse(raw.trim());
    if (parsed == null || parsed < 1 || parsed > _maxCount) {
      _controller.text = '${widget.pending ?? widget.value}';
      return;
    }
    if (parsed == widget.value) {
      widget.onRevert();
    } else {
      widget.onChanged(parsed);
    }
  }

  @override
  Widget build(BuildContext context) {
    final dirty = widget.pending != null && widget.pending != widget.value;
    return TextField(
      controller: _controller,
      enabled: widget.enabled,
      keyboardType: TextInputType.number,
      inputFormatters: [FilteringTextInputFormatter.digitsOnly],
      textAlign: TextAlign.end,
      decoration: InputDecoration(
        isDense: true,
        border: const OutlineInputBorder(),
        suffixIcon: dirty
            ? IconButton(
                icon: const Icon(Icons.undo, size: 16),
                onPressed: widget.onRevert,
              )
            : null,
      ),
      onSubmitted: _submit,
      onTapOutside: (_) => _submit(_controller.text),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message({
    required this.icon,
    required this.title,
    required this.body,
    this.onRetry,
  });

  final IconData icon;
  final String title;
  final String body;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 40, color: theme.colorScheme.outline),
            const SizedBox(height: 12),
            Text(title, style: theme.textTheme.titleMedium),
            const SizedBox(height: 8),
            Text(body, textAlign: TextAlign.center, style: theme.textTheme.bodyMedium),
            if (onRetry != null) ...[
              const SizedBox(height: 12),
              OutlinedButton(onPressed: onRetry, child: const Text('Retry')),
            ],
          ],
        ),
      ),
    );
  }
}
