import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/features/editor/ui/add_inventory_item_dialog.dart';
import 'package:goresave/features/editor/ui/game_time_dialog.dart';
import 'package:goresave/features/editor/ui/inventory_item_visual.dart';
import 'package:goresave/features/editor/ui/item_stats_tooltip.dart';
import 'package:goresave/features/editor/ui/pending_structural_row.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// The Handel (trade) sub-tab: what a merchant offers and how much ore he has
/// to buy with.
///
/// This is NOT his inventory. A merchant's shop lives in a global array keyed by
/// his unique name, and it carries two maps — the live stock and saved runtime
/// restock input. His ore sits inside the same map as an ordinary line,
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

/// How much of a short pane the panel's fixed head may keep before it scrolls,
/// so the stock browser below it always gets a usable slice.
const double _minBrowserHeight = 260;

double _headCap(double available) {
  if (!available.isFinite) return double.infinity;
  final cap = available - _minBrowserHeight;
  return cap > 0 ? cap : 0;
}

String _localizedResourcesLevel(AppLocalizations l10n, String level) =>
    switch (level) {
      'Novice' => l10n.presetNovice,
      'Gothic' => l10n.presetGothic,
      'Hard' => l10n.presetHard,
      _ => level,
    };

class _TraderPanelState extends ConsumerState<TraderPanel> {
  TradersResult? _list;
  TraderDetail? _detail;
  GameTime? _gameTime;
  String? _error;

  /// Several trader records carry this character's name, so none of them may be
  /// edited: the index an edit is addressed by would be a guess.
  bool _ambiguous = false;
  bool _loading = true;

  /// Guards against a slow reload landing after a newer one: only the newest
  /// epoch may write to the state.
  int _epoch = 0;

  /// Which category the sidebar has selected. Null until the first build picks
  /// one, and reset whenever the selection no longer has any lines.
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
      _gameTime = null;
      _ambiguous = false;
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
      // Either not a merchant, or a name several records carry — which is not
      // the same thing and must not read as one.
      setState(() {
        _loading = false;
        _list = list;
        _detail = null;
        _ambiguous = list.isAmbiguous(widget.actor.uniqueName);
      });
      return;
    }
    final detail = await widget.notifier.loadTraderDetail(row.index);
    if (!mounted || epoch != _epoch) return;
    final gameTime = widget.inspection.privateDecoded
        ? await widget.notifier.loadGameTime()
        : null;
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _loading = false;
      _list = list;
      _error = detail.error;
      _detail = detail.detail;
      _gameTime = gameTime;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Rebuild for replacements too: changing an already-pending world clock
    // keeps the map length stable but must update the restock forecast.
    ref.watch(editorProvider.select((s) => s.pendingEdits));

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
        icon: _ambiguous
            ? Icons.warning_amber_outlined
            : Icons.storefront_outlined,
        title: l10n.tabTrade,
        body: _ambiguous ? l10n.traderAmbiguousName : l10n.traderNotAMerchant,
      );
    }

    final list = _list;
    // Per-difficulty stock is not modelled. A save that carries it could accept
    // an edit to m_Items and later replace it from that other stock, so nothing
    // here is editable then.
    // Two shapes the editor cannot honour: per-difficulty stock it does not
    // model, and a record missing a stock list it cannot create.
    final incomplete = !detail.summary.stockMapsPresent;
    final unsupported = detail.hasItemsByDifficulty || incomplete;
    final coreCanSet =
        widget.editable && !unsupported && (list?.canSetStock ?? false);
    final coreCanAdd =
        widget.editable && !unsupported && (list?.canAddItem ?? false);
    final coreCanRemove =
        widget.editable && !unsupported && (list?.canRemoveItem ?? false);
    const map = TraderStockMap.current;
    final canSet = coreCanSet;
    final canAdd = coreCanAdd;
    final canRemove = coreCanRemove;
    final resourcesLevel = widget.notifier.activeResourcesLevelForRestock();
    final restockDays = resourcesLevel == null
        ? null
        : traderRestockDays(resourcesLevel);
    final activitySeconds = _pendingActivitySeconds(detail);
    final worldSeconds =
        widget.notifier.pendingGameTimeSeconds() ?? _gameTime?.totalSeconds;
    final timing = restockDays == null
        ? null
        : TraderRestockTiming(
            activitySeconds: activitySeconds,
            worldSeconds: worldSeconds,
            intervalDays: restockDays,
          );

    // The live stock gets the ore its own card, because that number is the
    // merchant's purchasing power and not just another line.
    final removals = _pendingRemovals(map);
    final rows = [
      for (final item in detail.items)
        if (!item.isOre && !removals.contains(item.path)) item,
    ];

    // The sub-tab layout every other detail pane uses, documented on
    // CharactersTab: outer 20/top 8, one card, 16 inside it. Trade used to lay
    // its blocks on the bare background, which made it the odd tab out.
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: LayoutBuilder(
            builder: (context, pane) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Notes and summary cards scroll among themselves once the
                  // pane gets short, so they cannot squeeze out the stock list.
                  ConstrainedBox(
                    constraints: BoxConstraints(
                      maxHeight: _headCap(pane.maxHeight),
                    ),
                    child: SingleChildScrollView(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          // First, because it qualifies every number below it — the ore as much
                          // as the stock counts.
                          if (unsupported) ...[
                            _NoteCard(
                              text: incomplete
                                  ? l10n.traderRecordIncomplete
                                  : l10n.traderDifficultyStockUnsupported,
                              tone: _NoteTone.warning,
                            ),
                            const SizedBox(height: 12),
                          ],
                          _NoteCard(
                            key: const ValueKey('trader-notes-card'),
                            text: l10n.traderPriceWarning,
                            warningText: l10n.traderCurrentStockWarning,
                          ),
                          // The core drops setStock from `writable` when no shop
                          // holds a line while still offering addItem, so "read only"
                          // has to mean none of the three is available — not merely
                          // that one of them is missing.
                          if (widget.editable &&
                              !unsupported &&
                              !coreCanSet &&
                              !coreCanAdd &&
                              !coreCanRemove) ...[
                            const SizedBox(height: 12),
                            Text(
                              l10n.traderReadOnlyCore,
                              style: theme.textTheme.bodySmall,
                            ),
                          ],
                          const SizedBox(height: 16),
                          LayoutBuilder(
                            builder: (context, cards) {
                              final sideBySide =
                                  cards.maxWidth >=
                                  _summaryCardsSideBySideAbove;
                              final availableWidth = sideBySide
                                  ? cards.maxWidth - 12
                                  : cards.maxWidth;
                              final oreWidth = sideBySide
                                  ? availableWidth * 0.4
                                  : availableWidth;
                              final restockWidth = sideBySide
                                  ? availableWidth - oreWidth
                                  : availableWidth;
                              final oreCard = _OreCard(
                                detail: detail,
                                editable: canSet,
                                canRemove: canRemove,
                                removalPending: removals.contains(
                                  kTraderOrePath,
                                ),
                                stacked: oreWidth - 32 < _oreStackBelow,
                                onChanged: (value) =>
                                    _queueSet(map, kTraderOrePath, value),
                                onRevert: () => _revert(map, kTraderOrePath),
                                onRemove: () =>
                                    _queueRemove(map, kTraderOrePath),
                                pending: _pendingCountFor(map, kTraderOrePath),
                              );
                              final restockCard = _RestockCard(
                                timing: timing,
                                activitySeconds: activitySeconds,
                                resourcesLevel: resourcesLevel,
                                stackFacts: restockWidth - 24 < 340,
                                stackControls: restockWidth - 24 < 260,
                                editable:
                                    widget.editable &&
                                    !unsupported &&
                                    detail.totalSecondsPath != null,
                                pending: _hasPendingActivityTime,
                                onSetNow: worldSeconds == null
                                    ? null
                                    : () => _queueActivityTime(worldSeconds),
                                onMakeDue:
                                    timing?.makeDueActivitySeconds == null
                                    ? null
                                    : () => _queueActivityTime(
                                        timing!.makeDueActivitySeconds!,
                                      ),
                                onCustom: _editActivityTime,
                                onRevert: _revertActivityTime,
                              );
                              if (!sideBySide) {
                                return Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.stretch,
                                  children: [
                                    oreCard,
                                    const SizedBox(height: 12),
                                    restockCard,
                                  ],
                                );
                              }
                              // Once they share a row, the taller card sets the
                              // row height and its neighbour stretches to match.
                              return IntrinsicHeight(
                                child: Row(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.stretch,
                                  children: [
                                    Expanded(flex: 2, child: oreCard),
                                    const SizedBox(width: 12),
                                    Expanded(flex: 3, child: restockCard),
                                  ],
                                ),
                              );
                            },
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  Expanded(
                    child: _StockSection(
                      map: map,
                      items: rows,
                      lineCount: detail.items.length,
                      pendingAdds: _pendingAdds(map),
                      pendingRemovals: [
                        for (final item in detail.items)
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
                      onAdd: () => _addItem(map, detail),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }

  int get _index => _detail!.summary.index;

  String get _activityPendingKey => 'traders:$_index:activityTime';

  TraderActivityTimeEdit? _activityEdit(double seconds) {
    final path = _detail?.totalSecondsPath;
    if (path == null) return null;
    return TraderActivityTimeEdit(
      index: _index,
      propertyPath: path,
      totalSeconds: seconds,
    );
  }

  bool get _hasPendingActivityTime =>
      widget.notifier.pendingEditFor(_activityPendingKey)?.edits.isNotEmpty ??
      false;

  double _pendingActivitySeconds(TraderDetail detail) {
    final pending = widget.notifier.pendingEditFor(
      'traders:${detail.summary.index}:activityTime',
    );
    final value = pending?.edits.firstOrNull?['value'];
    final seconds = value is Map ? value['value'] : null;
    return seconds is num ? seconds.toDouble() : detail.summary.totalSeconds;
  }

  void _queueActivityTime(double seconds) {
    final edit = _activityEdit(seconds);
    if (edit == null || !seconds.isFinite || seconds < 0) return;
    final original = _detail!.summary.totalSeconds;
    if (seconds == original) {
      _revertActivityTime();
      return;
    }
    widget.notifier.setTraderActivityTimeEdit(edit);
    setState(() {});
  }

  void _revertActivityTime() {
    final edit = _activityEdit(0);
    if (edit == null) return;
    widget.notifier.clearTraderActivityTimeEdit(edit);
    setState(() {});
  }

  Future<void> _editActivityTime() async {
    final detail = _detail;
    if (detail == null) return;
    final current = _pendingActivitySeconds(detail);
    final fallback =
        widget.notifier.pendingGameTimeSeconds() ??
        _gameTime?.totalSeconds ??
        0;
    final initial = GameTimeParts.fromTotalSeconds(
      current >= 0 ? current : fallback,
    );
    final edited = await showDialog<GameTimeParts>(
      context: context,
      builder: (_) => GameTimeDialog(
        initialValue: initial,
        title: AppLocalizations.of(context).traderRestockEditTitle,
      ),
    );
    if (!mounted || edited == null) return;
    // The dialog shows whole seconds. Treat an unchanged display value as a
    // no-op so confirming it cannot discard a stored sub-second fraction (or
    // replace an existing pending value with its truncated representation).
    // A never-active merchant is different: its displayed value is the world
    // time fallback, not the stored sentinel, so confirming must queue it.
    if (current >= 0 && edited.toTotalSeconds() == initial.toTotalSeconds()) {
      return;
    }
    _queueActivityTime(edited.toTotalSeconds().toDouble());
  }

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
    widget.notifier.clearTraderStockEdit(
      _edit(TraderEditKind.setStock, map, path),
    );
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
      builder: (_) => AddInventoryItemDialog(
        excludePaths: held,
        warningText: AppLocalizations.of(context).traderCurrentStockWarning,
      ),
    );
    if (result == null) return;
    if (!mounted || widget.notifier.selectedPath != savePath) return;
    widget.notifier.setTraderStockEdit(
      _edit(TraderEditKind.addItem, map, result.path, count: result.count),
    );
    setState(() {});
  }
}

/// Below this width the ore card's field and delete button no longer fit beside
/// the text, so they move under it.
const double _oreStackBelow = 320;

/// Below this width a stock row's value no longer fits beside its name, so it
/// moves under it. A ListTile gives its trailing whatever width it asks for, so
/// the row has to stop using one.
const double _rowStackBelow = 300;

/// Above this width the ore and restock summaries share one row. Each card
/// remains wide enough for its own compact layout; below it the Wrap stacks.
const double _summaryCardsSideBySideAbove = 580;

/// From this pane width compact actions carry their labels; under it they show
/// icons with tooltips instead.
const double _labelledActionsAbove = 320;

class _OreCard extends ConsumerWidget {
  const _OreCard({
    required this.detail,
    required this.editable,
    required this.canRemove,
    required this.removalPending,
    required this.stacked,
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
  final bool stacked;
  final void Function(int) onChanged;
  final VoidCallback onRevert;
  final VoidCallback onRemove;
  final int? pending;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final ore = detail.summary.ore;
    final label = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Flexible(
              child: Text(l10n.traderOre, style: theme.textTheme.titleMedium),
            ),
            const SizedBox(width: 6),
            Tooltip(
              key: const ValueKey('trader-ore-info'),
              message: l10n.traderOreHint,
              child: Icon(
                Icons.info_outline,
                size: 17,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(l10n.traderOreHintShort, style: theme.textTheme.bodySmall),
      ],
    );
    final field = ore == null
        // No ore line at all is a real state and NOT the same as zero, so say
        // so instead of showing a 0 the save does not contain.
        ? Text(l10n.traderNoOre, style: theme.textTheme.bodyMedium)
        : _CountField(
            value: ore,
            pending: pending,
            // While a removal is queued the number is on its way out; editing
            // it would queue a count for a line about to go.
            enabled: editable && !removalPending,
            onChanged: onChanged,
            onRevert: onRevert,
          );
    final delete = ore != null && canRemove
        ? IconButton(
            tooltip: l10n.traderRemoveItem,
            icon: const Icon(Icons.delete_outline, size: 20),
            onPressed: removalPending ? null : onRemove,
          )
        : null;
    return DecoratedBox(
      // Named so a test can reach this block without matching on the sheet
      // every panel now sits on.
      key: const ValueKey('trader-ore-card'),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            InventoryItemVisual(
              itemId: 'ItMi_Orenugget',
              itemPath: kTraderOrePath,
              fallbackIcon: Icons.savings_outlined,
              fallbackColor: theme.colorScheme.primary,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: stacked
                  ? Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        label,
                        const SizedBox(height: 12),
                        // The field takes what is left rather than a fixed
                        // width: stacked, there may be very little.
                        Row(
                          children: [
                            Expanded(child: field),
                            ?delete,
                          ],
                        ),
                      ],
                    )
                  : label,
            ),
            if (!stacked) ...[
              const SizedBox(width: 12),
              SizedBox(width: 140, child: field),
              ?delete,
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
  const _NoteCard({
    super.key,
    required this.text,
    this.warningText,
    this.tone = _NoteTone.info,
  });

  final String text;
  final String? warningText;
  final _NoteTone tone;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isWarning = tone == _NoteTone.warning;

    Widget line(String value, {required bool warning}) => Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(
          warning ? Icons.warning_amber_outlined : Icons.info_outline,
          size: 18,
          color: warning
              ? (isWarning
                    ? theme.colorScheme.onErrorContainer
                    : theme.colorScheme.error)
              : theme.colorScheme.primary,
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            value,
            style: theme.textTheme.bodySmall?.copyWith(
              color: isWarning ? theme.colorScheme.onErrorContainer : null,
            ),
          ),
        ),
      ],
    );

    // A block ON the sheet, not a second card lying on top of it.
    return DecoratedBox(
      decoration: BoxDecoration(
        color: isWarning
            ? theme.colorScheme.errorContainer
            : theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            line(text, warning: isWarning),
            if (warningText case final warning?) ...[
              const SizedBox(height: 8),
              line(warning, warning: true),
            ],
          ],
        ),
      ),
    );
  }
}

class _RestockCard extends StatelessWidget {
  const _RestockCard({
    required this.timing,
    required this.activitySeconds,
    required this.resourcesLevel,
    required this.stackFacts,
    required this.stackControls,
    required this.editable,
    required this.pending,
    required this.onSetNow,
    required this.onMakeDue,
    required this.onCustom,
    required this.onRevert,
  });

  final TraderRestockTiming? timing;
  final double activitySeconds;
  final String? resourcesLevel;
  final bool stackFacts;
  final bool stackControls;
  final bool editable;
  final bool pending;
  final VoidCallback? onSetNow;
  final VoidCallback? onMakeDue;
  final VoidCallback onCustom;
  final VoidCallback onRevert;

  String _clock(AppLocalizations l10n, double seconds) {
    final parts = GameTimeParts.fromTotalSeconds(seconds);
    final clock = [
      parts.hour,
      parts.minute,
      parts.second,
    ].map((value) => value.toString().padLeft(2, '0')).join(':');
    return '${l10n.gameTimeDay} ${parts.day} · $clock';
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final forecast = timing;
    final activity =
        activitySeconds > kTraderNeverActiveSeconds &&
            activitySeconds.isFinite &&
            activitySeconds >= 0
        ? _clock(l10n, activitySeconds)
        : l10n.traderRestockNever;
    final early = forecast?.calendarBoundarySeconds;
    final conservative = forecast?.elapsedBoundarySeconds;
    final window = early == null || conservative == null
        ? l10n.traderRestockUnavailable
        : early == conservative
        ? _clock(l10n, early)
        : '${_clock(l10n, early)} – ${_clock(l10n, conservative)}';
    final interval = resourcesLevel == null || forecast == null
        ? l10n.traderRestockIntervalUnknown
        : l10n.traderRestockInterval(
            forecast.intervalDays,
            _localizedResourcesLevel(l10n, resourcesLevel!),
          );
    final forecastState = forecast?.state;
    final statusText = resourcesLevel == null
        ? l10n.traderRestockStatusUnknown
        : switch (forecastState) {
            TraderRestockForecastState.neverActive =>
              l10n.traderRestockStatusNever,
            TraderRestockForecastState.clockAhead =>
              l10n.traderRestockStatusCheckTime,
            TraderRestockForecastState.beforeWindow =>
              l10n.traderRestockStatusWaiting,
            TraderRestockForecastState.boundaryOnly =>
              l10n.traderRestockStatusPossiblyReady,
            TraderRestockForecastState.eligibleBoth =>
              l10n.traderRestockStatusReady,
            _ => l10n.traderRestockStatusUnknown,
          };
    final statusIcon = switch (forecastState) {
      TraderRestockForecastState.neverActive => Icons.history_toggle_off,
      TraderRestockForecastState.clockAhead => Icons.warning_amber_outlined,
      TraderRestockForecastState.beforeWindow => Icons.hourglass_bottom,
      TraderRestockForecastState.boundaryOnly => Icons.help_outline,
      TraderRestockForecastState.eligibleBoth => Icons.check_circle_outline,
      _ => Icons.help_outline,
    };
    final statusColor = switch (forecastState) {
      TraderRestockForecastState.eligibleBoth => scheme.primary,
      TraderRestockForecastState.clockAhead ||
      TraderRestockForecastState.boundaryOnly => scheme.tertiary,
      _ => scheme.onSurfaceVariant,
    };

    Widget action({
      required Key key,
      required IconData icon,
      required String label,
      required String tooltip,
      required VoidCallback? onPressed,
    }) {
      final callback = editable ? onPressed : null;
      final button = IconButton.outlined(
        key: key,
        constraints: const BoxConstraints.tightFor(width: 36, height: 36),
        padding: EdgeInsets.zero,
        visualDensity: VisualDensity.compact,
        onPressed: callback,
        icon: Icon(icon, size: 19),
      );
      return Tooltip(
        message: tooltip,
        child: Semantics(label: label, button: true, child: button),
      );
    }

    final actionRow = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        action(
          key: const ValueKey('trader-restock-set-now'),
          icon: Icons.sync,
          label: l10n.traderRestockSetNow,
          tooltip: l10n.traderRestockSetNowTooltip,
          onPressed: onSetNow,
        ),
        const SizedBox(width: 2),
        action(
          key: const ValueKey('trader-restock-make-due'),
          icon: Icons.notification_important_outlined,
          label: l10n.traderRestockMakeDue,
          tooltip: l10n.traderRestockMakeDueTooltip,
          onPressed: onMakeDue,
        ),
        const SizedBox(width: 2),
        action(
          key: const ValueKey('trader-restock-custom'),
          icon: Icons.edit_calendar_outlined,
          label: l10n.traderRestockCustom,
          tooltip: l10n.traderRestockCustomTooltip,
          onPressed: onCustom,
        ),
      ],
    );
    List<Widget> titleChildren() => [
      const Icon(Icons.update_outlined, size: 20),
      const SizedBox(width: 8),
      Expanded(
        child: Tooltip(
          message: l10n.traderRestockTitleTooltip,
          child: Text(
            l10n.traderRestockTitle,
            style: theme.textTheme.titleSmall,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ),
      if (pending)
        Tooltip(
          message: l10n.traderRestockRevertTooltip,
          child: IconButton(
            key: const ValueKey('trader-restock-revert'),
            visualDensity: VisualDensity.compact,
            onPressed: onRevert,
            icon: const Icon(Icons.undo, size: 18),
          ),
        ),
    ];
    final header = stackControls
        ? Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(children: titleChildren()),
              const SizedBox(height: 4),
              Align(alignment: Alignment.centerRight, child: actionRow),
            ],
          )
        : Row(
            children: [...titleChildren(), const SizedBox(width: 4), actionRow],
          );

    return DecoratedBox(
      key: const ValueKey('trader-restock-card'),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: scheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            header,
            const SizedBox(height: 10),
            Semantics(
              key: const ValueKey('trader-restock-status'),
              label: '${l10n.traderRestockStatusLabel}: $statusText',
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  Text(
                    l10n.traderRestockStatusLabel,
                    style: theme.textTheme.labelMedium,
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        Icon(statusIcon, size: 19, color: statusColor),
                        const SizedBox(width: 6),
                        Flexible(
                          child: Text(
                            statusText,
                            style: theme.textTheme.bodySmall,
                            textAlign: TextAlign.end,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 8),
            _RestockFact(
              label: l10n.traderRestockLastActivity,
              tooltip: l10n.traderRestockLastActivityTooltip,
              value: activity,
              stacked: stackFacts,
            ),
            const SizedBox(height: 6),
            _RestockFact(
              label: l10n.traderRestockForecastWindow,
              tooltip: l10n.traderRestockForecastWindowTooltip,
              value: window,
              stacked: stackFacts,
            ),
            const SizedBox(height: 6),
            _RestockFact(
              label: l10n.traderRestockIntervalLabel,
              tooltip: l10n.traderRestockIntervalTooltip,
              value: interval,
              stacked: stackFacts,
            ),
          ],
        ),
      ),
    );
  }
}

class _RestockFact extends StatelessWidget {
  const _RestockFact({
    required this.label,
    required this.tooltip,
    required this.value,
    required this.stacked,
  });

  final String label;
  final String tooltip;
  final String value;
  final bool stacked;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final labelText = Tooltip(
      message: tooltip,
      child: Text(label, style: theme.textTheme.labelMedium),
    );
    if (stacked) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          labelText,
          const SizedBox(height: 2),
          Text(value, style: theme.textTheme.bodySmall),
        ],
      );
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(width: 150, child: labelText),
        const SizedBox(width: 10),
        Expanded(
          child: Text(
            value,
            style: theme.textTheme.bodySmall,
            textAlign: TextAlign.end,
          ),
        ),
      ],
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
  /// Below this the category rail folds away and the lines are shown flat.
  ///
  /// The rail costs 200 plus a 16 gap, and a line stays horizontal down to
  /// [_rowStackBelow], so 560 still leaves the lines more room than they need.
  /// Deliberately under 600: the shared sub-tab padding takes 40 of the pane,
  /// which at a 1400-wide window left 588 and folded the rail away on a screen
  /// with room for it.
  static const double _compactBelow = 560;

  /// The share of the pane the queued-change banners may claim before they
  /// scroll among themselves, so the stock browser stays visible below them.
  static const double _pendingMaxFraction = 0.4;

  /// Never more than this, however tall the pane is — past a few banners the
  /// rest may as well scroll.
  static const double _pendingMaxHeight = 240;

  /// Room the header and a usable slice of the list keep for themselves. On a
  /// pane too short to grant even that, the banners give way rather than push
  /// the column past its bounds.
  static const double _pendingReserve = 140;

  /// The height the banner strip may occupy inside a pane of [available].
  ///
  /// Measured against the pane, not against a constant: a fixed cap ignores the
  /// header above and the list below, so a short window or a large UI scale
  /// overflowed the column and collapsed the browser to nothing.
  static double _bannerCap(double available) {
    if (!available.isFinite) return _pendingMaxHeight;
    final byFraction = available * _pendingMaxFraction;
    final byReserve = available - _pendingReserve;
    final cap = byFraction < byReserve ? byFraction : byReserve;
    if (cap > _pendingMaxHeight) return _pendingMaxHeight;
    return cap > 0 ? cap : 0;
  }

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
    // The trader's stock is inventory too, so it is filed by the same tabs the
    // game's own inventory rail uses.
    final itemStats = ref.watch(itemStatsCatalogProvider).value;
    final groups = _grouped(items, displayNameOf: nameOf, stats: itemStats);
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

    // The compact pane has no sidebar, so it lists every line at once. The core
    // hands them over in class-id order, which in most languages is not the
    // order of the names on screen — sort them the way the groups are sorted.
    final flat = [...items]..sort(_byDisplayName(nameOf));
    // Hold the chosen category while it still has lines; otherwise fall back to
    // the first one so the list is never blank next to a populated sidebar.
    final selected = groups.any((g) => g.category == selectedCategory)
        ? selectedCategory
        : (groups.isEmpty ? null : groups.first.category);
    final shown =
        groups.where((g) => g.category == selected).firstOrNull?.items ??
        const <TraderItem>[];
    // "Nothing in stock" means the MAP is empty, not the filtered view: the ore
    // is pulled out of the live stock into its own card, so a merchant holding
    // only ore has a line and a purse on screen and must not be told otherwise.
    final mapIsEmpty =
        lineCount == 0 && pendingAdds.isEmpty && pendingRemovals.isEmpty;

    return LayoutBuilder(
      builder: (context, pane) => Column(
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
              // The label goes when the pane cannot carry it: at the smallest
              // supported window this button alone is wider than the row.
              if (canAdd)
                pane.maxWidth >= _labelledActionsAbove
                    ? OutlinedButton.icon(
                        icon: const Icon(Icons.add),
                        label: Text(l10n.traderAddItem),
                        onPressed: onAdd,
                      )
                    : IconButton.outlined(
                        icon: const Icon(Icons.add),
                        tooltip: l10n.traderAddItem,
                        onPressed: onAdd,
                      ),
            ],
          ),
          // Queued changes sit ABOVE the list: they are what the next save will
          // do, while the list below is what the save holds right now. Bounded and
          // scrollable, because replacing most of a merchant's stock queues enough
          // of them to push the browser off screen — and then the very rows that
          // cancel them become unreachable.
          if (pendingAdds.isNotEmpty || pendingRemovals.isNotEmpty)
            ConstrainedBox(
              constraints: BoxConstraints(
                maxHeight: _bannerCap(pane.maxHeight),
              ),
              child: SingleChildScrollView(
                child: Column(
                  children: [
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
                  ],
                ),
              ),
            ),
          const SizedBox(height: 8),
          if (mapIsEmpty)
            Align(
              alignment: Alignment.centerLeft,
              child: Text(
                l10n.traderEmptyStock,
                style: theme.textTheme.bodyMedium,
              ),
            )
          else if (items.isEmpty)
            // Nothing left to browse — every line this view would show sits in
            // the ore card or in the banners above.
            const SizedBox.shrink()
          else
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final compact = constraints.maxWidth < _compactBelow;
                  final rows = compact ? flat : shown;
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
                                      gameIcon:
                                          filtersById[group.category]?.icon ??
                                          gameIconForItemCategory(
                                            group.category,
                                          ),
                                      label: l10n.categoryWithCount(
                                        categoryLabel(group.category),
                                        group.items.length,
                                      ),
                                      selected: group.category == selected,
                                      onTap: () =>
                                          onSelectCategory(group.category),
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
                          // The same upper bound the inventory rows carry: a
                          // ListTile otherwise runs across the whole detail
                          // pane and pins its count field to the far edge,
                          // half a screen from the name it belongs to. Only a
                          // bound — a narrow pane still gets the full width.
                          itemBuilder: (context, index) => Align(
                            alignment: Alignment.centerLeft,
                            child: ConstrainedBox(
                              constraints: const BoxConstraints(maxWidth: 560),
                              child: _StockRow(
                                // The map belongs in the key: the same item exists in
                                // both, so keying on the path alone reused one row's
                                // field across a map switch — and with the focused
                                // guard skipping the sync, the old count stayed on
                                // screen while keystrokes went to the other map.
                                key: ValueKey((map, rows[index].path)),
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
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
        ],
      ),
    );
  }
}

/// One category's lines, in [ItemCategory] declaration order.
class _StockGroup {
  const _StockGroup({required this.category, required this.items});

  final ItemCategory category;
  final List<TraderItem> items;
}

/// Case-insensitively by the localized name the user reads, with the class id
/// as a stable tiebreak. Shared so the grouped pane and the compact one, which
/// has no sidebar to group by, agree on an order.
int Function(TraderItem, TraderItem) _byDisplayName(
  String Function(TraderItem item) displayNameOf,
) => (a, b) {
  final byName = displayNameOf(
    a,
  ).toLowerCase().compareTo(displayNameOf(b).toLowerCase());
  return byName != 0 ? byName : a.id.compareTo(b.id);
};

/// Group a stock map the way the inventory groups its own items — same
/// classifier, so a sword lands under Melee weapons in both places, and the same
/// sort: case-insensitively by the localized name the user reads, with the class
/// id as a stable tiebreak.
List<_StockGroup> _grouped(
  List<TraderItem> items, {
  required String Function(TraderItem item) displayNameOf,
  ItemStatsCatalog? stats,
}) {
  final byCategory = <ItemCategory, List<TraderItem>>{};
  for (final item in items) {
    byCategory
        .putIfAbsent(itemCategoryFor(item.id, stats: stats), () => [])
        .add(item);
  }
  final compare = _byDisplayName(displayNameOf);

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

    final field = _CountField(
      value: item.count,
      pending: pending,
      // An unknown class is shown but never edited: we cannot vouch for what
      // the game does with a line it does not recognise.
      enabled: canSet && !item.unknownItem,
      onChanged: onChanged,
      onRevert: onRevert,
    );
    final delete = canRemove
        ? IconButton(
            tooltip: l10n.traderRemoveItem,
            icon: const Icon(Icons.delete_outline, size: 20),
            onPressed: onRemove,
          )
        : null;
    final icon = InventoryItemVisual(
      key: ValueKey(('trader-item-image', map, item.path, item.id)),
      itemId: item.id,
      itemPath: item.path,
      fallbackIcon: item.isOre
          ? Icons.savings_outlined
          : Icons.inventory_2_outlined,
      fallbackColor: item.isOre ? theme.colorScheme.primary : null,
    );
    final text = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(label),
        if (subtitle.isNotEmpty)
          Text(subtitle, style: theme.textTheme.bodySmall),
      ],
    );

    // Hovering a row shows what the game shows when the player hovers the item,
    // the same card the inventory rows carry. Ore is the shop's buying power
    // rather than a stocked item, so it has no card.
    Widget hoverable(Widget row) => item.isOre
        ? row
        : ItemStatsTooltip(itemId: item.id, title: label, child: row);

    return LayoutBuilder(
      builder: (context, box) {
        // A ListTile keeps its trailing at full width, and the field plus the
        // delete button want ~180px — more than the whole row gets at the
        // smallest supported window. There the value moves under the name.
        if (box.maxWidth >= _rowStackBelow) {
          return hoverable(
            ListTile(
              dense: true,
              // Matched to the inventory rows: the count editor is a 48px
              // control, and dropping the tile's own vertical padding keeps
              // neighbouring rows compact without shrinking it.
              minTileHeight: 48,
              minVerticalPadding: 0,
              contentPadding: const EdgeInsets.symmetric(horizontal: 8),
              horizontalTitleGap: 8,
              leading: icon,
              title: Text(label),
              subtitle: subtitle.isEmpty
                  ? null
                  : Text(subtitle, style: theme.textTheme.bodySmall),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  SizedBox(width: 132, child: field),
                  ?delete,
                ],
              ),
            ),
          );
        }
        return hoverable(
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    icon,
                    const SizedBox(width: 12),
                    Expanded(child: text),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(child: field),
                    ?delete,
                  ],
                ),
              ],
            ),
          ),
        );
      },
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
  /// Shown under the field while the typed value cannot be queued.
  String? _error;

  /// Whether the user is in this field. While they are, its text belongs to
  /// them: every sync below is a reaction to a change they just made.
  final FocusNode _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    _focus.addListener(_onFocusChanged);
  }

  /// Put the field back in step the moment the user leaves it.
  ///
  /// While focused the text is theirs and no sync runs, so an emptied or
  /// refused entry would otherwise stay on screen afterwards — showing nothing,
  /// or a number the save never took, with no pending edit and no revert
  /// control to explain it.
  /// The field's own undo, which has to put the text back itself.
  ///
  /// The sync that normally would is deliberately off while the field has
  /// focus, so without this the discarded count stayed on screen with nothing
  /// queued behind it — and Save disabled — until the user clicked away.
  void _undo() {
    final saved = '${widget.value}';
    if (_controller.text != saved) _controller.text = saved;
    if (_error != null) setState(() => _error = null);
    widget.onRevert();
  }

  void _onFocusChanged() {
    if (_focus.hasFocus) return;
    final shown = '${widget.pending ?? widget.value}';
    if (_controller.text != shown) _controller.text = shown;
    if (_error != null) setState(() => _error = null);
  }

  late final TextEditingController _controller = TextEditingController(
    text: '${widget.pending ?? widget.value}',
  );

  @override
  void didUpdateWidget(covariant _CountField oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Never while the user is typing: backspacing through a queued count clears
    // the pending value, and restoring the saved one here put it straight back
    // under their cursor — leaving no way to empty the field and start over.
    if (_focus.hasFocus) return;
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
    _focus.removeListener(_onFocusChanged);
    _controller.dispose();
    _focus.dispose();
    super.dispose();
  }

  /// The core stores a count as an `i32`, so anything larger is refused at save
  /// time. The add-item dialog already caps at the same value.
  static const int _maxCount = 2147483647; // i32::MAX

  /// Queue on every keystroke, the way the inventory's count editor does.
  ///
  /// Waiting for Enter or a tap outside left a typed amount unregistered: Save
  /// stayed disabled while it sat in the field, and a rebuild could overwrite
  /// the text before it was ever queued — so the change simply never happened.
  /// An invalid entry says so in place and withdraws the queued edit rather
  /// than snapping the field back under the user's cursor.
  void _onChanged(String raw) {
    final l10n = AppLocalizations.of(context);
    final trimmed = raw.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = null);
      widget.onRevert();
      return;
    }
    final parsed = int.tryParse(trimmed);
    if (parsed == null || parsed < 1) {
      // Min 1: a sold-out line is deleted, not held at zero. The delete button
      // is how a line goes away.
      setState(() => _error = l10n.min1);
      widget.onRevert();
      return;
    }
    if (parsed > _maxCount) {
      setState(() => _error = l10n.countMustBeAtMost(_maxCount));
      widget.onRevert();
      return;
    }
    setState(() => _error = null);
    if (parsed == widget.value) {
      widget.onRevert();
    } else {
      widget.onChanged(parsed);
    }
  }

  @override
  Widget build(BuildContext context) {
    final dirty = widget.pending != null && widget.pending != widget.value;
    // The same field the inventory rows carry: named, theme-bordered and left
    // aligned. A stock count and an inventory count are the same kind of
    // number, and the two lists sit one tab apart.
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 48),
      child: TextField(
        controller: _controller,
        focusNode: _focus,
        enabled: widget.enabled,
        keyboardType: TextInputType.number,
        inputFormatters: [FilteringTextInputFormatter.digitsOnly],
        decoration: InputDecoration(
          labelText: AppLocalizations.of(context).count,
          isDense: true,
          errorText: _error,
          suffixIcon: dirty
              ? IconButton(
                  icon: const Icon(Icons.undo, size: 16),
                  onPressed: _undo,
                )
              : null,
        ),
        onChanged: _onChanged,
      ),
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
            Text(
              body,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium,
            ),
            if (onRetry != null) ...[
              const SizedBox(height: 12),
              OutlinedButton(
                onPressed: onRetry,
                child: Text(AppLocalizations.of(context).traderRetry),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
