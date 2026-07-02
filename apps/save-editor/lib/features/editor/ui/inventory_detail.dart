import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/actor_detail_header.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import '../domain/editor_notifier.dart';
import 'add_inventory_item_dialog.dart';

/// Resolves an inventory [PrivateInventorySummary] into the editable /
/// addable / removable gates the card consumes. [typedVerified] and
/// [canCompress] are save-wide; [writable] is the inventory's advertised op
/// list (the player's from the inspection, an NPC's from its loaded summary).
({bool editable, bool canAddItem, bool canRemoveItem}) _inventoryGates({
  required List<String> writable,
  required bool privateEditable,
  required bool typedVerified,
  required bool canCompress,
}) {
  final editable =
      privateEditable &&
      canCompress &&
      writable.contains('private.inventory.setItemCount');
  final canAddItem =
      privateEditable &&
      canCompress &&
      typedVerified &&
      writable.contains('private.inventory.addItem');
  final canRemoveItem =
      privateEditable &&
      canCompress &&
      typedVerified &&
      writable.contains('private.inventory.removeItem');
  return (
    editable: editable,
    canAddItem: canAddItem,
    canRemoveItem: canRemoveItem,
  );
}

/// The "Inventory" DETAIL body (everything to the right of the shared character
/// master list). When the Player is selected the existing inventory card path
/// is shown UNCHANGED (loaded from the inspection, edits with no actorId). When
/// an NPC is selected its inventory is loaded via `private.npc.inventory` and
/// fed to the SAME card widget, with edits keyed PER-NPC and carrying the NPC's
/// actorId. Selection is passed in via [actor] (the shared editor state) so it
/// stays in sync with the other character sub-tabs. Extracted verbatim from the
/// old `_InventoryPanel`.
class InventoryDetail extends ConsumerWidget {
  const InventoryDetail({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.actor,
    this.canCompress = false,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool canCompress;

  /// The selected actor (player or NPC). Orphans are guarded out by the caller,
  /// so this is only ever the player or a spawned NPC.
  final Actor actor;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    if (!inspection.privateDecoded) {
      return _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: l10n.inventoryTitle,
        body: l10n.inventoryNeedsDecoded,
      );
    }

    final selected = actor;
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};

    final Widget body;
    if (selected.isPlayer) {
      body = _playerInventoryDetail(context);
    } else {
      // NPC → load its inventory and feed the SAME card, keyed per-NPC so
      // switching NPCs never clobbers either one's queued edits.
      final npcId = selected.id!;
      body = _NpcInventoryDetail(
        // Reload when the inspected save OR the selected NPC changes.
        key: ValueKey(('npc-inventory', npcId)),
        reloadKey: (inspection, npcId),
        npcId: npcId,
        notifier: notifier,
        privateEditable: inspection.privateEditable,
        typedVerified: inspection.privateTypedVerified,
        canCompress: canCompress,
      );
    }

    // Shared header above the inventory body, identifying the selected actor
    // (NPC name + full GlobalId, or "Player") — same widget the Attribute tab
    // uses, so both tabs make the selection obvious.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ActorDetailHeader(
          actor: selected,
          locCatalog: locCatalog,
          lang: lang,
        ),
        Expanded(child: body),
      ],
    );
  }

  /// The existing player inventory detail — load from the inspection, edits
  /// under the literal `'inventory'` key with no actorId. Behaviour is
  /// byte-for-byte unchanged from before the shared-selector refactor.
  Widget _playerInventoryDetail(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final gates = _inventoryGates(
      writable: inspection.privateInventory.writable,
      privateEditable: inspection.privateEditable,
      typedVerified: inspection.privateTypedVerified,
      canCompress: canCompress,
    );
    final hasItems = inspection.privateInventory.hasData;
    if (!hasItems && !gates.canAddItem && !gates.canRemoveItem) {
      return _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: l10n.inventoryTitle,
        body: l10n.inventoryNoStacks,
      );
    }
    return Padding(
      padding: const EdgeInsets.all(20),
      child: _PrivateInventorySummaryCard(
        inventory: inspection.privateInventory,
        notifier: notifier,
        editable: gates.editable,
        canAddItem: gates.canAddItem,
        canRemoveItem: gates.canRemoveItem,
      ),
    );
  }
}

/// Loads the selected NPC's inventory via `private.npc.inventory` and renders
/// it with the SAME [_PrivateInventorySummaryCard] the player uses. Edits are
/// keyed PER-NPC (`'inventory:$npcId'`) and stamped with the NPC's actorId.
class _NpcInventoryDetail extends StatefulWidget {
  const _NpcInventoryDetail({
    super.key,
    required this.reloadKey,
    required this.npcId,
    required this.notifier,
    required this.privateEditable,
    required this.typedVerified,
    required this.canCompress,
  });

  final Object reloadKey;
  final String npcId;
  final EditorNotifier notifier;
  final bool privateEditable;
  final bool typedVerified;
  final bool canCompress;

  @override
  State<_NpcInventoryDetail> createState() => _NpcInventoryDetailState();
}

class _NpcInventoryDetailState extends State<_NpcInventoryDetail> {
  NpcInventoryResult? _result;
  bool _loading = false;
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant _NpcInventoryDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
  }

  Future<void> _reload() async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final result = await widget.notifier.loadNpcInventory(widget.npcId);
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _result = result;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    if (_loading || _result == null) {
      return const Padding(
        padding: EdgeInsets.all(24),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    final result = _result!;
    if (result.error != null) {
      return Padding(
        padding: const EdgeInsets.all(20),
        child: Text(
          result.error!,
          style: TextStyle(color: theme.colorScheme.error),
        ),
      );
    }

    final inventory = result.inventory;
    final gates = _inventoryGates(
      writable: inventory.writable,
      privateEditable: widget.privateEditable,
      typedVerified: widget.typedVerified,
      canCompress: widget.canCompress,
    );
    final hasItems = inventory.hasData;
    if (!hasItems && !gates.canAddItem && !gates.canRemoveItem) {
      return _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: l10n.inventoryTitle,
        body: l10n.inventoryNoStacks,
      );
    }
    return Padding(
      padding: const EdgeInsets.all(20),
      child: _PrivateInventorySummaryCard(
        // Key on the NPC so switching NPCs builds a fresh card state (its
        // local draft/pending fields reset) while the previous NPC's registry
        // entry survives under its own per-NPC key.
        key: ValueKey(('npc-inventory-card', widget.npcId)),
        inventory: inventory,
        notifier: widget.notifier,
        editable: gates.editable,
        canAddItem: gates.canAddItem,
        canRemoveItem: gates.canRemoveItem,
        pendingKey: 'inventory:${widget.npcId}',
        actorId: widget.npcId,
      ),
    );
  }
}

class _PrivateInventorySummaryCard extends ConsumerStatefulWidget {
  const _PrivateInventorySummaryCard({
    super.key,
    required this.inventory,
    required this.notifier,
    this.editable = true,
    this.canAddItem = false,
    this.canRemoveItem = false,
    this.pendingKey = 'inventory',
    this.actorId,
  });

  final PrivateInventorySummary inventory;
  final EditorNotifier notifier;
  final bool editable;
  final bool canAddItem;
  final bool canRemoveItem;

  /// Pending-edit registry key this card writes to. The player inventory uses
  /// the literal `'inventory'`; each NPC uses a PER-NPC key (`'inventory:$id'`)
  /// so editing NPC-A then NPC-B never clobbers either one's queued edits.
  final String pendingKey;

  /// GlobalId of the NPC whose inventory this edits, or null for the player.
  /// Forwarded onto every queued edit as `actorId` so the core targets the
  /// right container.
  final String? actorId;

  @override
  ConsumerState<_PrivateInventorySummaryCard> createState() =>
      _PrivateInventorySummaryCardState();
}

class _PrivateInventorySummaryCardState
    extends ConsumerState<_PrivateInventorySummaryCard> {
  String _query = '';
  final TextEditingController _searchController = TextEditingController();
  final Map<String, InventoryItemCountChange> _pendingCountChanges = {};
  ItemCategory? _selectedCategory;
  InventoryItemAdd? _pendingAdd;
  // The item queued for removal (carries path + slotId + containerType so the
  // core can target the right container's exact slot). addItem and removeItem
  // are both structural edits, so at most one of _pendingAdd / _pendingRemove is
  // ever set (the core allows one structural inventory edit per write).
  PrivateInventoryItem? _pendingRemove;

  /// Asset path of the item queued for removal, or null when none is queued.
  /// Kept as a thin accessor so the existing path-based list filtering / display
  /// is unchanged while [_pendingRemove] carries the full slot addressing.
  String? get _pendingRemovePath => _pendingRemove?.path;

  @override
  void initState() {
    super.initState();
    // Rehydrate any draft already queued for THIS actor under the per-NPC
    // pending key. Switching away from an edited NPC disposes this card's local
    // state but leaves the registry entry (`inventory:<npcId>`) intact;
    // revisiting builds a fresh card (keyed on the NPC) with empty local state.
    // Without this, the next edit would call setPendingEdit with ONLY the new
    // local edit and silently drop the earlier queued ones. Reading the stored
    // edits back into local state makes a revisit resume from the queued draft.
    _rehydrateFromPending();
  }

  /// Reconstruct local pending state from the registry entry for [pendingKey],
  /// reversing the JSON edits [_pushInventoryPending] writes. Inverse of
  /// [InventoryItemCountChange.toEditJson] et al. Tolerant of unexpected shapes
  /// (skips anything it can't parse) so a malformed entry never throws here.
  void _rehydrateFromPending() {
    final entry = widget.notifier.pendingEditFor(widget.pendingKey);
    if (entry == null) return;
    for (final edit in entry.edits) {
      final path = edit['path'];
      final value = edit['value'];
      if (value is! Map) continue;
      switch (path) {
        case 'private.inventory.setItemCount':
          final itemPath = value['path'] as String? ?? '';
          final count = (value['count'] as num?)?.toInt();
          if (itemPath.isEmpty || count == null) continue;
          final id = value['id'] as String? ?? '';
          final slotId = (value['slotId'] as num?)?.toInt();
          final containerType = value['containerType'] as String?;
          final item = PrivateInventoryItem(
            id: id,
            path: itemPath,
            slotId: slotId,
            containerType: containerType,
          );
          _pendingCountChanges[_inventoryItemKey(item)] =
              InventoryItemCountChange(
            id: id,
            path: itemPath,
            count: count,
            slotId: slotId,
            containerType: containerType,
          );
        case 'private.inventory.addItem':
          final itemPath = value['path'] as String? ?? '';
          final count = (value['count'] as num?)?.toInt();
          if (itemPath.isEmpty || count == null) continue;
          _pendingAdd = InventoryItemAdd(path: itemPath, count: count);
        case 'private.inventory.removeItem':
          final itemPath = value['path'] as String? ?? '';
          if (itemPath.isEmpty) continue;
          _pendingRemove = PrivateInventoryItem(
            id: value['id'] as String? ?? '',
            path: itemPath,
            slotId: (value['slotId'] as num?)?.toInt(),
            containerType: value['containerType'] as String?,
          );
      }
    }
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant _PrivateInventorySummaryCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.inventory != widget.inventory) {
      // Inventory was refreshed — rebuild local widget state from the registry.
      // A FULL save clears 'inventory:<id>' centrally, so rehydrate finds nothing
      // and the drafts stay cleared. A PARTIALLY-committed save re-applies the
      // still-uncommitted pending edits to the registry, so rehydrate restores
      // them here — otherwise the Save badge would count queued edits while the
      // count fields showed on-disk values, and the next edit would overwrite the
      // registry with only that field. Rehydrate only mutates LOCAL state (never
      // the provider), so it is safe in this build-context callback.
      _pendingCountChanges.clear();
      _pendingAdd = null;
      _pendingRemove = null;
      _rehydrateFromPending();
    }
  }

  void _pushInventoryPending() {
    final actorId = widget.actorId;
    // Stamp the selected actor onto every queued edit. actorId is null for the
    // player (key omitted from the JSON, behaviour byte-for-byte unchanged) and
    // the NPC's GlobalId otherwise (core targets the NPC's container).
    final countEdits = _pendingCountChanges.values
        .map(
          (c) => InventoryItemCountChange(
            id: c.id,
            path: c.path,
            count: c.count,
            actorId: actorId,
            slotId: c.slotId,
            containerType: c.containerType,
          ).toEditJson(),
        )
        .toList();
    final addEdit = _pendingAdd != null
        ? InventoryItemAdd(
            path: _pendingAdd!.path,
            count: _pendingAdd!.count,
            actorId: actorId,
          ).toEditJson()
        : null;
    final removeEdit = _pendingRemove != null
        ? InventoryItemRemove(
            path: _pendingRemove!.path,
            actorId: actorId,
            slotId: _pendingRemove!.slotId,
            containerType: _pendingRemove!.containerType,
          ).toEditJson()
        : null;
    final allEdits = [
      ...countEdits,
      ?addEdit,
      ?removeEdit,
    ];
    if (allEdits.isEmpty) {
      widget.notifier.clearPendingEdit(widget.pendingKey);
    } else {
      widget.notifier.setPendingEdit(
        widget.pendingKey,
        PendingSaveEdit(edits: allEdits),
      );
    }
  }

  Future<void> _openAddDialog() async {
    // Scope the dialog to the save it was opened for. If the user switches to a
    // different save while the dialog is open, the awaited result is stale — its
    // excludePaths and target belong to the old save, so applying it would queue
    // the add against the wrong save. Key on the selected save path, not the
    // inventory object identity: re-inspecting the SAME save allocates a fresh
    // summary instance with identical contents, and that result must still apply.
    final dialogSavePath = widget.notifier.selectedPath;
    final result = await showDialog<InventoryItemAdd>(
      context: context,
      builder: (_) => AddInventoryItemDialog(
        // Exclude both MainContainer paths (addItem rejects duplicates there)
        // and the currently-equipped armor: adding a second copy of the worn
        // armor would duplicate its definition path and make the equipped
        // badge/upgrades ambiguous, so it must not be offered. Both lists are
        // uncapped (from the typed tree), so the exclusion is correct even when
        // the displayed row list is truncated.
        excludePaths: {
          ...widget.inventory.mainContainerPaths,
          ...widget.inventory.equippedArmorPaths,
        },
      ),
    );
    if (result == null) return;
    if (!mounted || widget.notifier.selectedPath != dialogSavePath) return;
    setState(() {
      _pendingAdd = result;
      // Keep the one-structural-edit-per-save invariant unconditionally.
      _pendingRemove = null;
    });
    _pushInventoryPending();
  }

  void _queueRemove(PrivateInventoryItem item) {
    setState(() {
      // A removal supersedes any pending count change on the same item, and is
      // mutually exclusive with a pending add (one structural edit per save).
      _pendingCountChanges.remove(_inventoryItemKey(item));
      _pendingAdd = null;
      // Carry the full item so the remove edit echoes slotId + containerType,
      // letting the core target the exact slot in the right container.
      _pendingRemove = item;
    });
    _pushInventoryPending();
  }

  void _undoRemove() {
    setState(() => _pendingRemove = null);
    _pushInventoryPending();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};
    final inventory = widget.inventory;
    final query = _query.trim().toLowerCase();
    final items = inventory.items.where((item) {
      // A pending removal hides ONLY the specific slot queued for removal (it is
      // represented by the pending card above), matched by the same slot-aware
      // key as count edits — so duplicate-path stacks, or the same item in
      // another container, are not all hidden when only one slot will be removed.
      if (_pendingRemove != null &&
          _inventoryItemKey(item) == _inventoryItemKey(_pendingRemove!)) {
        return false;
      }
      if (query.isEmpty) return true;
      final name = localizedGameName(locCatalog, lang, item.id);
      return item.id.toLowerCase().contains(query) ||
          item.path.toLowerCase().contains(query) ||
          (name != null && name.toLowerCase().contains(query));
    }).toList();
    final groups = groupInventoryItems(items);

    // Keep the current category selected if it still has items, else fall
    // back to the first available group.
    var selected = _selectedCategory;
    if (groups.every((g) => g.category != selected)) {
      selected = groups.isEmpty ? null : groups.first.category;
    }
    final selectedGroup =
        groups.where((g) => g.category == selected).firstOrNull;

    // An active search shows matches across all categories as a flat list;
    // an empty query browses by the selected category.
    final searching = query.isNotEmpty;
    final shownItems = searching
        ? items
        : (selectedGroup?.items ?? const <PrivateInventoryItem>[]);

    final hasItems = inventory.items.isNotEmpty;
    final hasPendingAdd = _pendingAdd != null;
    final hasPendingRemove = _pendingRemovePath != null;
    final hasPendingCount = _pendingCountChanges.isNotEmpty;
    final hasPendingChanges =
        hasPendingCount || hasPendingAdd || hasPendingRemove;
    final canRemove = widget.canRemoveItem;
    // A structural edit (add/remove) must be saved on its own (the core/notifier
    // reject a batch that mixes it with count edits), so structural edits and
    // count edits are kept mutually exclusive in the UI: a structural edit is
    // blocked while counts are pending, and count editing is blocked while a
    // structural edit is pending.
    final structuralBlocked = hasPendingAdd || hasPendingRemove || hasPendingCount;
    final countEditable = widget.editable && !hasPendingAdd && !hasPendingRemove;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.inventory_2_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.inventoryTitle,
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && hasPendingChanges) ...[
                  Tooltip(
                    message: l10n.resetInventoryChanges,
                    child: IconButton(
                      icon: const Icon(Icons.undo_outlined),
                      onPressed: () {
                        setState(() {
                          _pendingCountChanges.clear();
                          _pendingAdd = null;
                          _pendingRemove = null;
                        });
                        widget.notifier.clearPendingEdit(widget.pendingKey);
                      },
                    ),
                  ),
                ],
                if (widget.canAddItem) ...[
                  const SizedBox(width: 8),
                  Tooltip(
                    message: hasPendingAdd
                        ? l10n.addItemTooltipPendingAdd
                        : hasPendingRemove
                            ? l10n.addItemTooltipPendingRemove
                            : hasPendingCount
                                ? l10n.addItemTooltipPendingCount
                                : l10n.addItemTooltipDefault,
                    child: FilledButton.icon(
                      icon: const Icon(Icons.add, size: 18),
                      label: Text(l10n.addItemButton),
                      onPressed: structuralBlocked ? null : _openAddDialog,
                    ),
                  ),
                ],
              ],
            ),
            if (hasPendingAdd) ...[
              const SizedBox(height: 12),
              _PendingStructuralRow(
                tone: _PendingTone.add,
                icon: Icons.add_circle_outline,
                title: _itemDisplayFromPath(_pendingAdd!.path),
                subtitle: l10n.pendingAddSubtitle(_pendingAdd!.count),
                cancelTooltip: l10n.cancelPendingAdd,
                onCancel: () {
                  setState(() => _pendingAdd = null);
                  _pushInventoryPending();
                },
              ),
            ],
            if (hasPendingRemove) ...[
              const SizedBox(height: 12),
              _PendingStructuralRow(
                tone: _PendingTone.remove,
                icon: Icons.delete_outline,
                title: _itemDisplayFromPath(_pendingRemovePath!),
                subtitle: l10n.pendingRemovalSubtitle,
                cancelTooltip: l10n.cancelPendingRemoval,
                onCancel: _undoRemove,
              ),
            ],
            if (hasItems) ...[
              const SizedBox(height: 12),
              TextField(
                controller: _searchController,
                decoration: InputDecoration(
                  labelText: l10n.filterItems,
                  prefixIcon: const Icon(Icons.search),
                ),
                onChanged: (value) => setState(() => _query = value),
              ),
              const SizedBox(height: 12),
              Expanded(
                child: groups.isEmpty
                    ? Center(
                        child: Text(
                          // An empty query with no rows means a pending removal
                          // hid the last item(s) — not a filter miss, so don't
                          // claim "no items match".
                          searching
                              ? l10n.noItemsMatchQuery(_query)
                              : l10n.pendingRemovalHidesAll,
                          style: theme.textTheme.bodyMedium,
                        ),
                      )
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
                                    for (final group in groups)
                                      SidebarTile(
                                        icon: iconForItemCategory(
                                          group.category,
                                        ),
                                        label: l10n.categoryWithCount(
                                          localizedItemCategoryLabel(
                                            l10n,
                                            group.category,
                                          ),
                                          group.items.length,
                                        ),
                                        selected: !searching &&
                                            group.category == selected,
                                        onTap: () => setState(() {
                                          _selectedCategory = group.category;
                                          // Leave search mode so the chosen
                                          // category's items are shown.
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
                            child: shownItems.isEmpty
                                ? const SizedBox.shrink()
                                : ListView.builder(
                                    itemCount: shownItems.length,
                                    itemBuilder: (context, index) {
                                      final item = shownItems[index];
                                      return ListTile(
                                        dense: true,
                                        leading: const Icon(
                                          Icons.category_outlined,
                                        ),
                                        title: Row(
                                          children: [
                                            Flexible(
                                              child: Text(
                                                localizedGameName(
                                                      locCatalog,
                                                      lang,
                                                      item.id,
                                                    ) ??
                                                    (item.id.isEmpty
                                                        ? item.path
                                                        : item.id),
                                                maxLines: 1,
                                                overflow: TextOverflow.ellipsis,
                                              ),
                                            ),
                                            if (item.equipped) ...[
                                              const SizedBox(width: 8),
                                              Container(
                                                padding:
                                                    const EdgeInsets.symmetric(
                                                      horizontal: 6,
                                                      vertical: 2,
                                                    ),
                                                decoration: BoxDecoration(
                                                  color: theme
                                                      .colorScheme
                                                      .primaryContainer,
                                                  borderRadius:
                                                      BorderRadius.circular(4),
                                                ),
                                                child: Text(
                                                  l10n.equippedBadge,
                                                  style: theme
                                                      .textTheme
                                                      .labelSmall
                                                      ?.copyWith(
                                                        color: theme
                                                            .colorScheme
                                                            .onPrimaryContainer,
                                                      ),
                                                ),
                                              ),
                                            ],
                                          ],
                                        ),
                                        subtitle:
                                            item.path.isEmpty &&
                                                item.upgrades.isEmpty
                                            ? null
                                            : Column(
                                                crossAxisAlignment:
                                                    CrossAxisAlignment.start,
                                                mainAxisSize: MainAxisSize.min,
                                                children: [
                                                  if (item.path.isNotEmpty)
                                                    Text(
                                                      item.path,
                                                      maxLines: 1,
                                                      overflow:
                                                          TextOverflow.ellipsis,
                                                    ),
                                                  if (item.upgrades.isNotEmpty)
                                                    Padding(
                                                      padding:
                                                          const EdgeInsets.only(
                                                            top: 4,
                                                          ),
                                                      child: Wrap(
                                                        spacing: 4,
                                                        runSpacing: 2,
                                                        crossAxisAlignment:
                                                            WrapCrossAlignment
                                                                .center,
                                                        children: [
                                                          Text(
                                                            l10n.armorUpgradesLabel,
                                                            style: theme
                                                                .textTheme
                                                                .labelSmall,
                                                          ),
                                                          for (final u
                                                              in item.upgrades)
                                                            Container(
                                                              padding:
                                                                  const EdgeInsets.symmetric(
                                                                    horizontal: 6,
                                                                    vertical: 1,
                                                                  ),
                                                              decoration: BoxDecoration(
                                                                color: theme
                                                                    .colorScheme
                                                                    .surfaceContainerHighest,
                                                                borderRadius:
                                                                    BorderRadius.circular(
                                                                      4,
                                                                    ),
                                                              ),
                                                              child: Text(
                                                                '${_upgradePart(u.key)}: ${_upgradeTier(u.value)}',
                                                                style: theme
                                                                    .textTheme
                                                                    .labelSmall
                                                                    ?.copyWith(
                                                                      color: theme
                                                                          .colorScheme
                                                                          .onSurfaceVariant,
                                                                    ),
                                                              ),
                                                            ),
                                                        ],
                                                      ),
                                                    ),
                                                ],
                                              ),
                                        trailing: _inventoryItemTrailing(
                                          theme,
                                          l10n,
                                          item,
                                          canRemove: canRemove,
                                          countEditable: countEditable,
                                          removeBlocked: structuralBlocked,
                                        ),
                                      );
                                    },
                                  ),
                          ),
                        ],
                      ),
              ),
            ] else if (!hasPendingAdd) ...[
              // Completely empty inventory (and nothing queued to add): say so
              // explicitly instead of showing a blank card.
              const SizedBox(height: 12),
              Text(
                l10n.inventoryEmpty,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  void _setPendingCountChange(
    PrivateInventoryItem item,
    InventoryItemCountChange? change,
  ) {
    setState(() {
      final key = _inventoryItemKey(item);
      if (change == null) {
        _pendingCountChanges.remove(key);
      } else {
        _pendingCountChanges[key] = change;
      }
    });
    _pushInventoryPending();
  }

  /// Trailing widget for an inventory row: count editor + a delete button.
  Widget _inventoryItemTrailing(
    ThemeData theme,
    AppLocalizations l10n,
    PrivateInventoryItem item, {
    required bool canRemove,
    required bool countEditable,
    required bool removeBlocked,
  }) {
    // The count editor shows only when count editing is currently allowed (the
    // inventory is count-editable AND no structural edit is pending). A
    // remove-only inventory, or one with a pending structural edit, shows the
    // count as plain text but the delete action may still apply.
    return Row(
      mainAxisSize: MainAxisSize.min,
      // Centre the delete button against the count field so the trash icon lines
      // up with the input value rather than floating up by the 'Count' label.
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        if (countEditable)
          _InventoryItemCountEditor(
            // Key by the slot-aware row key so duplicate-path/id stacks that
            // differ only by slotId never share a controller (which could leave
            // one row showing the other slot's count).
            key: ValueKey(_inventoryItemKey(item)),
            item: item,
            pendingCount: _pendingCountChanges[_inventoryItemKey(item)]?.count,
            onPendingCountChanged: (change) =>
                _setPendingCountChange(item, change),
          )
        else
          Text(
            l10n.countTimes('${item.count ?? '?'}'),
            style: theme.textTheme.bodyMedium,
          ),
        if (canRemove && item.path.isNotEmpty) ...[
          const SizedBox(width: 4),
          Tooltip(
            message: !item.removable
                ? l10n.deleteEquippedTooltip
                : removeBlocked
                ? l10n.removeBlockedTooltip
                : l10n.removeItemFromInventory,
            child: IconButton(
              icon: const Icon(Icons.delete_outline),
              // A non-removable item shows the trash icon disabled (its asset
              // path occurs in more than one container — e.g. also equipped or
              // in a quickslot — so the core can't unambiguously remove it). A
              // removable item is disabled only while a structural/count edit
              // is pending; otherwise it queues the remove.
              onPressed: (!item.removable || removeBlocked)
                  ? null
                  : () => _queueRemove(item),
            ),
          ),
        ],
      ],
    );
  }
}

/// Tone of a pending structural-edit card: an add (primary) or a remove
/// (error).
enum _PendingTone { add, remove }

/// A human-readable id fragment derived from an item asset path.
String _itemDisplayFromPath(String path) =>
    path.contains('.') ? path.split('.').last : path.split('/').last;

String _upgradePart(String key) {
  if (key.contains('Upper')) return 'Upper';
  if (key.contains('Mid')) return 'Mid';
  if (key.contains('Lower')) return 'Lower';
  return key;
}

String _upgradeTier(String value) {
  var v = value;
  for (final p in const ['m_UpperBody_', 'm_MidBody_', 'm_LowerBody_']) {
    if (v.startsWith(p)) {
      v = v.substring(p.length);
      break;
    }
  }
  return v.replaceAll('_ArmorUpgrade', '');
}

/// A highlighted card shown when there is a pending structural inventory edit
/// (add or remove) awaiting save. Mirrors how a not-yet-saved item is
/// represented for both directions: the affected item is not shown inline, only
/// here, with a cancel button.
class _PendingStructuralRow extends StatelessWidget {
  const _PendingStructuralRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onCancel,
    required this.cancelTooltip,
    required this.tone,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onCancel;
  final String cancelTooltip;
  final _PendingTone tone;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final isAdd = tone == _PendingTone.add;
    final bg = isAdd ? scheme.primaryContainer : scheme.errorContainer;
    final fg = isAdd ? scheme.onPrimaryContainer : scheme.onErrorContainer;
    final accent = isAdd ? scheme.primary : scheme.error;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: accent.withValues(alpha: 0.4)),
      ),
      child: ListTile(
        dense: true,
        leading: Icon(icon, color: accent),
        title: Text(
          title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(color: fg),
        ),
        subtitle: Text(
          subtitle,
          style: TextStyle(color: fg.withValues(alpha: 0.8)),
        ),
        trailing: IconButton(
          icon: const Icon(Icons.close),
          tooltip: cancelTooltip,
          onPressed: onCancel,
        ),
      ),
    );
  }
}

class _InventoryItemCountEditor extends StatefulWidget {
  const _InventoryItemCountEditor({
    super.key,
    required this.item,
    required this.onPendingCountChanged,
    this.pendingCount,
  });

  final PrivateInventoryItem item;
  final int? pendingCount;
  final void Function(InventoryItemCountChange? change) onPendingCountChanged;

  @override
  State<_InventoryItemCountEditor> createState() =>
      _InventoryItemCountEditorState();
}

class _InventoryItemCountEditorState extends State<_InventoryItemCountEditor> {
  late final TextEditingController _controller;
  String? _path;
  String? _id;
  int? _slotId;
  int? _count;
  int? _pendingCount;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _InventoryItemCountEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    // Rows are identified by path-or-id PLUS slotId — two duplicate stacks share
    // a path/id and differ only by slotId, and the on-disk count can change under
    // a fixed slot after a refresh — so include slotId and the canonical count;
    // otherwise the early-return could leave the field showing a stale count.
    if (_path == widget.item.path &&
        _id == widget.item.id &&
        _slotId == widget.item.slotId &&
        _count == widget.item.count &&
        _pendingCount == widget.pendingCount) {
      return;
    }
    final isSameItem = _path == widget.item.path &&
        _id == widget.item.id &&
        _slotId == widget.item.slotId;
    _path = widget.item.path;
    _id = widget.item.id;
    _slotId = widget.item.slotId;
    _count = widget.item.count;
    _pendingCount = widget.pendingCount;
    final text = (widget.pendingCount ?? widget.item.count)?.toString() ?? '';
    if (_controller.text != text) {
      final currentOffset = _controller.selection.baseOffset;
      final nextOffset = isSameItem
          ? currentOffset.clamp(0, text.length)
          : text.length;
      _controller.value = TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: nextOffset),
      );
    }
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 132,
      child: TextField(
        controller: _controller,
        keyboardType: TextInputType.number,
        onChanged: _onCountTextChanged,
        decoration: InputDecoration(
          labelText: AppLocalizations.of(context).count,
          errorText: _error,
          // Compact the field so it fits inside the dense ListTile row. A
          // reserved helper line would steal the input box's vertical space
          // here (the tile caps the field height), squeezing the box until the
          // value clips at the border — so the error grows the row instead.
          isDense: true,
        ),
      ),
    );
  }

  void _onCountTextChanged(String value) {
    final trimmed = value.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = null);
      widget.onPendingCountChanged(null);
      return;
    }
    final parsed = int.tryParse(trimmed);
    if (parsed == null || parsed < 1) {
      // Min 1: a count of 0 would leave a ghost slot (invisible in-game but
      // still in the save). Use the remove button to delete an item.
      final min1 = AppLocalizations.of(context).min1;
      setState(() => _error = min1);
      widget.onPendingCountChanged(null);
      return;
    }
    setState(() => _error = null);
    if (parsed == widget.item.count) {
      widget.onPendingCountChanged(null);
      return;
    }
    widget.onPendingCountChanged(
      InventoryItemCountChange(
        id: widget.item.id,
        path: widget.item.path,
        count: parsed,
        slotId: widget.item.slotId,
        containerType: widget.item.containerType,
      ),
    );
  }
}

String _inventoryItemKey(PrivateInventoryItem item) {
  // Combine container type, slot id, id, and path so rows that share a
  // definition path get distinct pending-change entries instead of collapsing
  // onto one key. The container type keeps identical paths in different
  // containers (e.g. a MeleeSlot sword and a Pouch stack sharing a slotId)
  // distinct; the stable slot id keeps two duplicate same-path/same-id stacks in
  // ONE container independent; id+path remains the fallback for rows without one.
  return '${item.containerType ?? ''}\u0000${item.slotId ?? ''}\u0000${item.id}\u0000${item.path}';
}

/// Centered icon + title + body message pane for empty/locked states. A private
/// copy of the same widget in `editor_page.dart` / `world_tab.dart`
/// (kept per-file so these detail widgets have no cross-file dependency).
class _MessagePane extends StatelessWidget {
  const _MessagePane({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  icon,
                  size: 48,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 12),
                Text(title, style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                Text(body, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
