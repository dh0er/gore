import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart'
    show HeroAttributeGroup, heroAttributeGroup, heroAttributeRank;
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/ui/grouped_attribute_sidebar.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart' show formatHeroValue;
import 'package:goresave/l10n/app_localizations.dart';

/// Optional NPC status wiring for [NpcAttributesPanel]. When supplied, a Status
/// row (`Status: <lebend|tot>` + a Revive action + HP readout) is rendered as
/// the FIRST entry of the core ("Hauptwerte") group detail. NPC-only — the
/// player view never supplies it.
class NpcStatusConfig {
  const NpcStatusConfig({
    required this.npcId,
    required this.reloadKey,
    required this.load,
    required this.onRevive,
    required this.editable,
    this.knownDead = false,
    this.revivePending = false,
  });

  /// The selected NPC's GlobalId. Used to pick the exact summary row out of the
  /// substring-filtered list result.
  final String npcId;

  /// The NPC's dead state known at selection time (from the actor sidebar).
  /// Fallback for the status row + Revive gate when the async summary [load]
  /// fails or the id is missing from the page, so a dead NPC never shows as
  /// alive with Revive disabled.
  final bool knownDead;

  /// Identifies the inspected save + NPC. When it changes the summary reloads,
  /// so a save's trailing refresh re-seeds the status line + HP from disk.
  final Object reloadKey;

  /// Loads the NPC summary page (the parent passes a closure over the cached
  /// full NPC list); the row picks the exact-id match.
  final Future<NpcActorsPage> Function() load;

  /// Register a pending revive (wired to `EditorNotifier.setPendingNpcRevive`).
  final VoidCallback onRevive;

  /// True when a revive is already queued for this NPC (a pending edit exists).
  /// Flips the status line optimistically + keeps the button enabled.
  final bool revivePending;

  /// Whether editing is allowed (gates the Revive button).
  final bool editable;
}

/// Grouped, editable attribute editor for a single NPC. Data arrives through
/// [load] (the core `private.npc.attributes` command) and pending edits leave
/// through [onPendingChanged] — the same load + pending-edit contract the
/// player's [HeroStatsCard] uses, so the parent registers the NPC edits via the
/// identical `private.typed.setValue` pending mechanism (with a distinct key).
///
/// NPC attribute keys share the player's id namespace (Health, MaxHealth,
/// Resistance_*, Strength, PickPocketing, …), so [heroAttributeGroup] classifies
/// them into the same Main stats / Combat / Resistances / Thieving / Advanced
/// groups as the player. NPC-specific extras (DamageMultiplier, SuperArmor,
/// Oxygen*, …) fall into the Advanced catch-all. This renders the SAME
/// master-detail sidebar as the player (via [GroupedAttributeSidebar]), showing
/// only groups that have at least one NPC attribute. Each row reuses the same
/// base/current number-field validation idiom as `_HeroAttributeRow` (mirrored
/// in [_NpcAttributeRow] below).
///
/// [reloadKey] identifies the inspected save+NPC: when it changes, the local
/// field drafts are dropped and the list reloads. The parent keys pending edits
/// per-NPC, so a prior NPC's registered edits survive the switch (they apply to
/// that NPC on save) — only this panel's in-progress text is cleared.
class NpcAttributesPanel extends StatefulWidget {
  const NpcAttributesPanel({
    super.key,
    required this.load,
    required this.onPendingChanged,
    required this.editable,
    required this.reloadKey,
    this.status,
    this.initialPending,
  });

  final Future<NpcAttributesResult> Function() load;

  /// Drafts already queued for this NPC under the parent's per-NPC pending key,
  /// supplied so a revisit RESUMES from them. Switching away from an edited NPC
  /// drops this panel's local `_pending` but leaves the parent's registry entry
  /// intact; without rehydrating, the next edit would notify with ONLY the new
  /// field and the parent would replace the stored entry, dropping the earlier
  /// edits. Evaluated on each (re)load so it reflects the current registry.
  final List<NpcTypedEdit> Function()? initialPending;

  /// When provided (NPC-only), a Status row (`Status: <lebend|tot>` + Revive
  /// + HP) is rendered as the FIRST entry of the core ("Hauptwerte") group
  /// detail. The player view never passes it.
  final NpcStatusConfig? status;

  /// Called whenever the set of pending edits changes. [edits] is the full list
  /// of typed edits to write (empty when nothing dirty/valid). [validationError]
  /// is non-null when any field is invalid or empty — the whole panel's edits
  /// are then suppressed until corrected.
  final void Function(List<NpcTypedEdit> edits, String? validationError)
  onPendingChanged;

  final bool editable;
  final Object reloadKey;

  @override
  State<NpcAttributesPanel> createState() => _NpcAttributesPanelState();
}

class _NpcAttributesPanelState extends State<NpcAttributesPanel> {
  List<NpcAttributeRow> _attributes = const [];
  String? _error;
  bool _loadFailed = false;
  bool _loading = false;
  // Pending field texts keyed by the typed path (joined). Cleared on reload.
  final Map<String, String> _pending = {};
  // Currently selected attribute group (sidebar pane).
  HeroAttributeGroup? _selected;
  // Epoch counter used to discard results from superseded reload calls.
  int _reloadEpoch = 0;

  // The NPC status snapshot (isDead + hp/maxHp) backing the Status row in the
  // core group. Loaded from the cached NPC list via [NpcStatusConfig.load] when
  // a status config is supplied (NPC-only). Null for the player view.
  NpcActor? _statusActor;
  bool _statusLoading = false;
  int _statusEpoch = 0;

  String _groupTitle(AppLocalizations l10n, HeroAttributeGroup g) => switch (g) {
    HeroAttributeGroup.core => l10n.heroGroupMainStats,
    HeroAttributeGroup.combat => l10n.heroGroupCombatSkills,
    HeroAttributeGroup.resistances => l10n.heroGroupResistances,
    HeroAttributeGroup.thieving => l10n.heroGroupThieving,
    HeroAttributeGroup.advanced => l10n.heroGroupAdvanced,
  };

  IconData _groupIcon(HeroAttributeGroup g) => switch (g) {
    HeroAttributeGroup.core => Icons.favorite_border,
    HeroAttributeGroup.combat => Icons.shield_outlined,
    HeroAttributeGroup.resistances => Icons.security_outlined,
    HeroAttributeGroup.thieving => Icons.key_outlined,
    HeroAttributeGroup.advanced => Icons.tune,
  };

  /// Group the loaded NPC rows by [heroAttributeGroup], ordering rows within a
  /// group by the shared [heroAttributeRank] (player's ordering).
  Map<HeroAttributeGroup, List<NpcAttributeRow>> _byGroup() {
    final byGroup = <HeroAttributeGroup, List<NpcAttributeRow>>{};
    for (final attribute in _attributes) {
      byGroup.putIfAbsent(heroAttributeGroup(attribute.key), () => [])
        .add(attribute);
    }
    for (final rows in byGroup.values) {
      rows.sort((a, b) {
        final rank = heroAttributeRank(a.key).compareTo(heroAttributeRank(b.key));
        return rank != 0 ? rank : a.key.compareTo(b.key);
      });
    }
    return byGroup;
  }

  @override
  void initState() {
    super.initState();
    _reload();
    _reloadStatus();
  }

  @override
  void didUpdateWidget(covariant NpcAttributesPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
    // Reload the status snapshot when the NPC identity changes, or when a status
    // config appears/disappears (player↔NPC) or its reloadKey changes (a save's
    // trailing refresh re-seeds the status line + HP from disk).
    if (widget.status?.reloadKey != oldWidget.status?.reloadKey) {
      _reloadStatus();
    }
  }

  /// Exact-id match of the status NPC within the loaded (full) NPC list, so the
  /// row reflects THIS NPC, not a same-prefix sibling.
  NpcActor? _matchStatusSelf(NpcActorsPage page, String npcId) {
    for (final npc in page.npcs) {
      if (npc.id == npcId) return npc;
    }
    return null;
  }

  Future<void> _reloadStatus() async {
    final status = widget.status;
    if (status == null) {
      setState(() {
        _statusActor = null;
        _statusLoading = false;
      });
      return;
    }
    final epoch = ++_statusEpoch;
    setState(() => _statusLoading = true);
    final page = await status.load();
    if (!mounted || epoch != _statusEpoch) return;
    setState(() {
      _statusLoading = false;
      _statusActor = _matchStatusSelf(page, status.npcId);
    });
  }

  Future<void> _reload() async {
    final epoch = ++_reloadEpoch;
    setState(() {
      _loading = true;
      _pending.clear();
      // Rehydrate this NPC's queued drafts so a revisit resumes from them
      // instead of starting empty (which would let the next edit replace the
      // parent's stored entry with only the newly-dirty field). Keyed by the
      // typed path exactly as _onFieldChanged keys live edits.
      final initial = widget.initialPending?.call() ?? const [];
      for (final edit in initial) {
        if (edit.path.isEmpty) continue;
        _pending[_pathKey(edit.path)] = formatHeroValue(edit.value);
      }
    });
    // Do NOT call onPendingChanged here. Calling it from
    // initState/didUpdateWidget would mutate the provider during build and throw
    // with flutter_riverpod — same constraint as HeroStatsCard._reload.
    //
    // Not notifying on reload is correct because pending edits are keyed
    // PER-NPC by the parent (`npc.attributes:$npcId`): switching to another NPC
    // rebuilds this panel with a new reloadKey and a DIFFERENT pending key, so
    // the previous NPC's registry entry must stay intact (it applies to THAT
    // NPC on save) — we only drop our local field drafts here. Within a single
    // save+NPC the entry is re-driven by the next onChanged, and a save/refresh
    // clears every key centrally.
    final result = await widget.load();
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _loadFailed = result.error != null;
      _attributes = result.attributes;
    });
  }

  String _pathKey(List<String> path) => path.join(' ');

  void _onFieldChanged(List<String> path, String text) {
    _pending[_pathKey(path)] = text;
    _recomputePending();
  }

  /// Recompute the pending edits from all dirty fields and notify the parent.
  /// On any validation error, notifies with empty edits + the error so the
  /// parent clears the NPC pending contribution. Mirrors
  /// `_HeroStatsCardState._recomputePending`.
  void _recomputePending() {
    final l10n = AppLocalizations.of(context);
    final edits = <NpcTypedEdit>[];
    for (final attribute in _attributes) {
      for (final (path, original) in [
        (attribute.basePath, attribute.base),
        (attribute.currentPath, attribute.current),
      ]) {
        if (path.isEmpty) continue;
        final text = _pending[_pathKey(path)];
        // Untouched fields have no pending entry and are no-ops.
        if (text == null) continue;
        // A cleared field is almost certainly an accident.
        if (text.trim().isEmpty) {
          final errMsg = l10n.attributeEmpty(attribute.key);
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        final value = double.tryParse(text.trim());
        if (value == null) {
          final errMsg = l10n.attributeInvalidNumber(attribute.key, text);
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        if (value == original) continue;
        edits.add(NpcTypedEdit(path: path, value: value));
      }
    }
    // All fields valid. Clear any prior validation error.
    if (_error != null && !_loadFailed) setState(() => _error = null);
    widget.onPendingChanged(edits, null);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_loading) {
      return const Padding(
        padding: EdgeInsets.all(24),
        child: Center(child: CircularProgressIndicator()),
      );
    }

    if (_loadFailed) {
      return Padding(
        padding: const EdgeInsets.all(20),
        child: Text(
          _error!,
          style: TextStyle(color: theme.colorScheme.error),
        ),
      );
    }

    final errorRow = _error != null
        ? Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              _error!,
              style: TextStyle(color: theme.colorScheme.error),
            ),
          )
        : null;

    // Group the NPC rows the same way the player view groups its attributes.
    final byGroup = _byGroup();
    final hasStatus = widget.status != null;
    // Only groups with at least one NPC attribute appear, in enum order. The
    // Thieving group is dropped for NPCs: no NPC ever carries a non-zero
    // thieving value (PickPocketing appears on many NPCs but is always 0;
    // Lockpick* never appear), so it would only ever surface dead 0-rows. The
    // player view (HeroStatsCard) keeps Thieving — this exclusion is NPC-only.
    //
    // When a status config is supplied the core ("Hauptwerte") group always
    // appears even with no core attributes, since the Status row lives at the
    // top of that group's detail and must have a home.
    final groups = HeroAttributeGroup.values
        .where((g) => g != HeroAttributeGroup.thieving)
        .where(
          (g) =>
              byGroup[g]?.isNotEmpty == true ||
              (hasStatus && g == HeroAttributeGroup.core),
        )
        .toList();

    if (groups.isEmpty) {
      // No attributes at all and no status — keep showing any inline error.
      return Padding(
        padding: const EdgeInsets.all(20),
        child: errorRow ?? const SizedBox.shrink(),
      );
    }

    // Default to the first available group (Main stats when present, since the
    // groups list is in enum order). Re-pick if the selection went stale after
    // a reload dropped its group.
    final effective = (_selected != null && groups.contains(_selected))
        ? _selected!
        : groups.first;

    Widget detailFor(HeroAttributeGroup group) {
      final attributes = byGroup[group] ?? const [];
      // The Status row is the FIRST entry of the core group detail (NPC-only).
      final statusRow = (hasStatus && group == HeroAttributeGroup.core)
          ? _buildStatusRow(context)
          : null;
      return Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ?errorRow,
            Card(
              child: Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(_groupIcon(group)),
                        const SizedBox(width: 8),
                        Text(
                          _groupTitle(AppLocalizations.of(context), group),
                          style: theme.textTheme.titleSmall,
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    if (statusRow != null) ...[
                      statusRow,
                      const Divider(height: 16),
                    ],
                    for (final a in attributes)
                      _NpcAttributeRow(
                        // A fresh reloadKey rebuilds every row so stale field
                        // drafts from a previous NPC never carry over.
                        key: ValueKey((widget.reloadKey, a.key, a.basePath)),
                        attribute: a,
                        editable: widget.editable,
                        initialBaseText: _pending[_pathKey(a.basePath)],
                        initialCurrentText: _pending[_pathKey(a.currentPath)],
                        onBaseChanged: (text) =>
                            _onFieldChanged(a.basePath, text),
                        onCurrentChanged: (text) =>
                            _onFieldChanged(a.currentPath, text),
                      ),
                  ],
                ),
              ),
            ),
          ],
        ),
      );
    }

    final l10n = AppLocalizations.of(context);

    return GroupedAttributeSidebar(
      selected: effective,
      onSelect: (id) {
        if (_selected != id) setState(() => _selected = id as HeroAttributeGroup);
      },
      panes: [
        for (final group in groups)
          SidebarPane(
            id: group,
            label: _groupTitle(l10n, group),
            icon: _groupIcon(group),
            detail: detailFor(group),
          ),
      ],
    );
  }

  /// The NPC Status row shown as the FIRST entry of the core ("Hauptwerte")
  /// group detail: `Status <lebend|tot>` on the left and a **Wiederbeleben**
  /// (Revive) action on the right. The HP readout was intentionally removed —
  /// the row shows only the alive/dead state and the Revive action.
  ///
  /// "lebend" when alive (`!isDead` and no pending revive), "tot" when dead. A
  /// queued revive is reflected optimistically: the status line flips to "wird
  /// wiederbelebt …" and the button shows the queued label, clearing on the
  /// post-save refresh. Revive is enabled only when the NPC `isDead` (or a
  /// revive is already pending) AND editing is allowed.
  Widget _buildStatusRow(BuildContext context) {
    final status = widget.status!;
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final actor = _statusActor;

    // Prefer the freshly-loaded summary; fall back to the dead state known from
    // the selection when the load failed / the id was missing (a dead NPC must
    // never read as alive, and Revive must stay reachable).
    final isDead = actor?.isDead ?? status.knownDead;
    final pending = status.revivePending;
    final canRevive = status.editable && (isDead || pending);

    final String stateText;
    final Color stateColor;
    if (pending) {
      stateText = l10n.npcReviveQueued;
      stateColor = scheme.primary;
    } else if (isDead) {
      stateText = l10n.npcStatusDead;
      stateColor = scheme.error;
    } else {
      stateText = l10n.npcStatusAlive;
      stateColor = scheme.onSurfaceVariant;
    }

    // Mirror the attribute-row grid so the state value lines up with the base
    // column: a 170-wide label cell, then the value in the first (base) column.
    // The left padding matches a TextField's content inset so the text is flush
    // with the base-field values below it. The Revive action sits in the second
    // (current) column.
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: 170,
            child: Text(
              l10n.npcStatusRowLabel,
              style: theme.textTheme.labelLarge,
            ),
          ),
          Expanded(
            child: _statusLoading
                ? const Align(
                    alignment: Alignment.centerLeft,
                    child: SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  )
                : Padding(
                    padding: const EdgeInsets.only(left: 12),
                    child: Text(
                      stateText,
                      style: theme.textTheme.bodyMedium
                          ?.copyWith(color: stateColor),
                    ),
                  ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                icon: const Icon(Icons.healing_outlined, size: 18),
                label: Text(l10n.npcReviveButton),
                onPressed: canRevive ? status.onRevive : null,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// One NPC attribute row: a label plus base/current number fields. Mirrors the
/// player's `_HeroAttributeRow` (same responsive Row/Column layout and
/// signed-decimal keyboard) but keyed off [NpcAttributeRow] data.
class _NpcAttributeRow extends StatefulWidget {
  const _NpcAttributeRow({
    super.key,
    required this.attribute,
    required this.editable,
    required this.onBaseChanged,
    required this.onCurrentChanged,
    this.initialBaseText,
    this.initialCurrentText,
  });

  final NpcAttributeRow attribute;
  final bool editable;
  final ValueChanged<String> onBaseChanged;
  final ValueChanged<String> onCurrentChanged;
  final String? initialBaseText;
  final String? initialCurrentText;

  @override
  State<_NpcAttributeRow> createState() => _NpcAttributeRowState();
}

class _NpcAttributeRowState extends State<_NpcAttributeRow> {
  late final TextEditingController _baseController;
  late final TextEditingController _currentController;

  @override
  void initState() {
    super.initState();
    _baseController = TextEditingController(
      text: widget.initialBaseText ?? formatHeroValue(widget.attribute.base),
    );
    _currentController = TextEditingController(
      text: widget.initialCurrentText ??
          formatHeroValue(widget.attribute.current),
    );
  }

  @override
  void dispose() {
    _baseController.dispose();
    _currentController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.attribute.key;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 620;
          final baseField = TextField(
            controller: _baseController,
            enabled: widget.editable,
            onChanged: widget.onBaseChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeBase(label),
            ),
          );
          final currentField = TextField(
            controller: _currentController,
            enabled: widget.editable,
            onChanged: widget.onCurrentChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeCurrent(label),
            ),
          );
          final rowLabel = Text(
            label,
            style: Theme.of(context).textTheme.labelLarge,
          );
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                rowLabel,
                const SizedBox(height: 6),
                baseField,
                const SizedBox(height: 6),
                currentField,
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(width: 170, child: rowLabel),
              Expanded(child: baseField),
              const SizedBox(width: 8),
              Expanded(child: currentField),
            ],
          );
        },
      ),
    );
  }
}
