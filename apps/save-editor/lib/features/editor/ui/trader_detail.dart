import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/features/editor/ui/add_inventory_item_dialog.dart';
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
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final Actor actor;

  /// Same save-wide gate the other editing panes take
  /// (`privateEditable && privateTypedVerified && codecCompressReady`).
  final bool editable;

  @override
  ConsumerState<TraderPanel> createState() => _TraderPanelState();
}

class _TraderPanelState extends ConsumerState<TraderPanel> {
  TradersResult? _list;
  TraderDetail? _detail;
  String? _error;
  bool _loading = true;

  /// Which save and actor the currently held data belongs to, so a reload that
  /// lands after the user moved on is discarded instead of shown.
  String? _loadedFor;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant TraderPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.actor.uniqueName != widget.actor.uniqueName ||
        oldWidget.notifier.selectedPath != widget.notifier.selectedPath) {
      _load();
    }
  }

  String get _token => '${widget.notifier.selectedPath}|${widget.actor.uniqueName}';

  Future<void> _load() async {
    final token = _token;
    setState(() {
      _loading = true;
      _error = null;
      _detail = null;
    });
    final list = await widget.notifier.loadTraders();
    if (!mounted || _token != token) return;
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
        _loadedFor = token;
      });
      return;
    }
    final detail = await widget.notifier.loadTraderDetail(row.index);
    if (!mounted || _token != token) return;
    setState(() {
      _loading = false;
      _list = list;
      _error = detail.error;
      _detail = detail.detail;
      _loadedFor = token;
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
    if (detail == null || _loadedFor != _token) {
      return _Message(
        icon: Icons.storefront_outlined,
        title: l10n.tabTrade,
        body: l10n.traderNotAMerchant,
      );
    }

    final list = _list;
    final canSet = widget.editable && (list?.canSetStock ?? false);
    final canAdd = widget.editable && (list?.canAddItem ?? false);
    final canRemove = widget.editable && (list?.canRemoveItem ?? false);

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
      children: [
        _OreCard(
          detail: detail,
          editable: canSet,
          onChanged: (value) => _queueSet(TraderStockMap.current, kTraderOrePath, value),
          onRevert: () => _revert(TraderStockMap.current, kTraderOrePath),
          pending: _pendingCountFor(TraderStockMap.current, kTraderOrePath),
        ),
        const SizedBox(height: 12),
        Card(
          margin: EdgeInsets.zero,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(Icons.info_outline, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.traderPriceWarning,
                    style: theme.textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
        ),
        if (widget.editable && !(list?.canSetStock ?? false)) ...[
          const SizedBox(height: 12),
          Text(l10n.traderReadOnlyCore, style: theme.textTheme.bodySmall),
        ],
        const SizedBox(height: 16),
        _StockSection(
          title: l10n.traderStockCurrent,
          hint: null,
          map: TraderStockMap.current,
          items: detail.items,
          canSet: canSet,
          canAdd: canAdd,
          canRemove: canRemove,
          pendingOf: _pendingCountFor,
          isRemovalPending: _isRemovalPending,
          onChanged: _queueSet,
          onRevert: _revert,
          onRemove: _queueRemove,
          onAdd: () => _addItem(TraderStockMap.current, detail),
        ),
        const SizedBox(height: 24),
        _StockSection(
          title: l10n.traderStockBase,
          hint: l10n.traderStockBaseHint,
          map: TraderStockMap.base,
          items: detail.defaultItems,
          canSet: canSet,
          canAdd: canAdd,
          canRemove: canRemove,
          pendingOf: _pendingCountFor,
          isRemovalPending: _isRemovalPending,
          onChanged: _queueSet,
          onRevert: _revert,
          onRemove: _queueRemove,
          onAdd: () => _addItem(TraderStockMap.base, detail),
        ),
      ],
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

  bool _isRemovalPending(TraderStockMap map, String path) {
    final key = _edit(TraderEditKind.removeItem, map, path).pendingKey;
    final pending = ref.read(editorProvider).pendingEdits[key];
    return pending?.edits.firstOrNull?['path'] == 'private.traders.removeItem';
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
    required this.onChanged,
    required this.onRevert,
    required this.pending,
  });

  final TraderDetail detail;
  final bool editable;
  final void Function(int) onChanged;
  final VoidCallback onRevert;
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
                  const SizedBox(height: 4),
                  Text(
                    detail.summary.traded
                        ? l10n.traderTraded
                        : l10n.traderNeverTraded,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.outline,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            if (ore == null)
              // No ore line at all is a real state and NOT the same as zero, so
              // say so instead of showing a 0 the save does not contain.
              Text(l10n.traderNoOre, style: theme.textTheme.bodyMedium)
            else
              SizedBox(
                width: 140,
                child: _CountField(
                  value: ore,
                  pending: pending,
                  enabled: editable,
                  onChanged: onChanged,
                  onRevert: onRevert,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _StockSection extends StatelessWidget {
  const _StockSection({
    required this.title,
    required this.hint,
    required this.map,
    required this.items,
    required this.canSet,
    required this.canAdd,
    required this.canRemove,
    required this.pendingOf,
    required this.isRemovalPending,
    required this.onChanged,
    required this.onRevert,
    required this.onRemove,
    required this.onAdd,
  });

  final String title;
  final String? hint;
  final TraderStockMap map;
  final List<TraderItem> items;
  final bool canSet;
  final bool canAdd;
  final bool canRemove;
  final int? Function(TraderStockMap, String) pendingOf;
  final bool Function(TraderStockMap, String) isRemovalPending;
  final void Function(TraderStockMap, String, int) onChanged;
  final void Function(TraderStockMap, String) onRevert;
  final void Function(TraderStockMap, String) onRemove;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.titleMedium),
                  Text(
                    l10n.traderStockLineCount(items.length),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.outline,
                    ),
                  ),
                  if (hint != null) ...[
                    const SizedBox(height: 4),
                    Text(hint!, style: theme.textTheme.bodySmall),
                  ],
                ],
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
        const SizedBox(height: 8),
        if (items.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(l10n.traderEmptyStock, style: theme.textTheme.bodyMedium),
          )
        else
          Card(
            margin: EdgeInsets.zero,
            child: Column(
              children: [
                for (final item in items)
                  _StockRow(
                    item: item,
                    map: map,
                    canSet: canSet,
                    canRemove: canRemove,
                    pending: pendingOf(map, item.path),
                    removalPending: isRemovalPending(map, item.path),
                    onChanged: (v) => onChanged(map, item.path, v),
                    onRevert: () => onRevert(map, item.path),
                    onRemove: () => onRemove(map, item.path),
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _StockRow extends ConsumerWidget {
  const _StockRow({
    required this.item,
    required this.map,
    required this.canSet,
    required this.canRemove,
    required this.pending,
    required this.removalPending,
    required this.onChanged,
    required this.onRevert,
    required this.onRemove,
  });

  final TraderItem item;
  final TraderStockMap map;
  final bool canSet;
  final bool canRemove;
  final int? pending;
  final bool removalPending;
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

    return ListTile(
      dense: true,
      leading: item.isOre
          ? Icon(Icons.savings_outlined, color: theme.colorScheme.primary)
          : const Icon(Icons.inventory_2_outlined),
      title: Text(
        label,
        style: removalPending
            ? theme.textTheme.bodyMedium?.copyWith(
                decoration: TextDecoration.lineThrough,
                color: theme.colorScheme.outline,
              )
            : null,
      ),
      subtitle: Text(
        item.unknownItem ? '${item.id} · ${l10n.traderUnknownItem}' : item.id,
        style: theme.textTheme.bodySmall,
      ),
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
              enabled: canSet && !removalPending && !item.unknownItem,
              onChanged: onChanged,
              onRevert: onRevert,
            ),
          ),
          if (canRemove)
            IconButton(
              tooltip: l10n.traderRemoveItem,
              icon: Icon(
                removalPending ? Icons.undo : Icons.delete_outline,
                size: 20,
              ),
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
    // Only overwrite when the field is not the thing that produced the change,
    // otherwise typing fights the controller.
    if (oldWidget.pending != widget.pending && '$shown' != _controller.text) {
      _controller.text = '$shown';
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit(String raw) {
    final parsed = int.tryParse(raw.trim());
    if (parsed == null || parsed < 0) {
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
