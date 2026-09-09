import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/domain/lock_catalog.dart';
import 'package:goresave/features/editor/ui/area_labels.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/locks_models.dart';
import '../domain/pending_edits.dart';

/// Localized item name for a key, falling back to the id-derived name the
/// inventory uses when no localization catalog has been extracted.
///
/// Shared by the rows and the search index: a key the reader sees named one way
/// and searches by another is the defect this single source removes.
String localizedKeyName(
  Map<String, Map<String, String>> locCatalog,
  GameLang lang,
  String id,
) =>
    localizedGameName(locCatalog, lang, id) ??
    itemDisplayNameFromId(id, fallback: id);

/// Which kinds of lock the list shows.
enum _KindFilter { all, chests, doors }

/// Which lock states the list shows.
enum _StateFilter { all, unlocked, locked }

/// One row of the list: a catalog entry joined with what the save says about it.
///
/// [entry] is null for a name the save carries that the bundled catalog does
/// not know. Those are kept and shown rather than dropped: the catalog is
/// cook-specific, so a save written by another build can name a lock this one
/// has no row for, and hiding it would hide a real, editable piece of state.
class _LockRow {
  _LockRow({required this.name, required this.entry, required this.unlocked});

  final String name;
  final LockEntry? entry;
  final bool unlocked;

  bool get isDoor => entry?.kind == LockKind.door;
  String get area => entry?.area ?? '';
  String get search => entry?.search ?? name.toLowerCase();
}

/// World-tab section listing every chest and door lock in the game, with the
/// save's own "already opened" state editable per row.
///
/// Two sources meet here. Which locks EXIST comes from the bundled catalog,
/// harvested from the game's script cache; which of them the player has opened
/// comes from `LockPersistentData.m_UnlockedLocks` in the save, which is the
/// only half a save carries at all — a fresh game has an empty set.
class LocksDetail extends ConsumerStatefulWidget {
  const LocksDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
    this.catalogOverride,
    this.locationsOverride,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  /// Replaces the bundled asset in widget tests; production leaves it null and
  /// gets [LockCatalog.loadBundled].
  final LockCatalog? catalogOverride;

  /// Same, for the location catalog the region rail takes its area names from.
  final LocationCatalog? locationsOverride;

  @override
  ConsumerState<LocksDetail> createState() => _LocksDetailState();
}

class _LocksDetailState extends ConsumerState<LocksDetail> {
  LocksResult _saved = LocksResult();
  LockCatalog? _catalog;
  LocationCatalog? _locations;
  bool _loading = false;
  int _reloadEpoch = 0;

  /// Queued toggles, keyed by the lock's own name. Toggling a row back to the
  /// value the save already holds REMOVES its entry instead of stacking a
  /// second one, which is what makes a second tap read as "undo".
  final Map<String, LockSetUnlockedEdit> _pending = {};

  String _query = '';
  String _area = '';
  Map<String, String> _searchIndex = const {};
  Object? _searchIndexKey;
  _KindFilter _kind = _KindFilter.all;
  _StateFilter _state = _StateFilter.all;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant LocksDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey ||
        !identical(widget.notifier, oldWidget.notifier)) {
      // A completed save re-seeds the inspection; drafts that survived it are
      // re-read from the registry in _load.
      _pending.clear();
      _load();
    }
  }

  Future<void> _load() async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final saved = await widget.notifier.loadLocks();
    if (!mounted || epoch != _reloadEpoch) return;
    final catalog = widget.catalogOverride ?? await LockCatalog.loadBundled();
    if (!mounted || epoch != _reloadEpoch) return;
    // Only for the area names in the rail; a failure here must not cost the
    // list, so an unreadable catalog degrades to bare area codes.
    LocationCatalog? locations = widget.locationsOverride;
    if (locations == null) {
      try {
        locations = await LocationCatalog.loadBundled();
      } catch (_) {
        locations = null;
      }
    }
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _saved = saved;
      _catalog = catalog;
      _locations = locations;
      _seedFromPending();
    });
  }

  /// Re-read queued toggles from the global registry, so a section switch (or a
  /// partial save) does not lose drafts the Save button would still write.
  void _seedFromPending() {
    _pending.clear();
    final queued = widget.notifier.pendingEditFor(
      EditorNotifier.pendingLocksKey,
    );
    for (final edit in queued?.edits ?? const <Map<String, Object?>>[]) {
      final value = edit['value'];
      if (value is! Map) continue;
      final lock = value['lock'];
      final unlocked = value['unlocked'];
      if (lock is String && unlocked is bool) {
        _pending[lock] = LockSetUnlockedEdit(lock: lock, unlocked: unlocked);
      }
    }
  }

  /// Push the whole draft map as ONE pending entry. Every toggle is
  /// value-addressed, so they batch into a single `write_save`.
  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit(EditorNotifier.pendingLocksKey);
      return;
    }
    widget.notifier.setPendingEdit(
      EditorNotifier.pendingLocksKey,
      PendingSaveEdit(
        edits: _pending.values.map((edit) => edit.toEditJson()).toList(),
      ),
    );
  }

  void _toggle(_LockRow row, bool unlocked) {
    setState(() {
      if (unlocked == row.unlocked) {
        // Back to what the save holds: the edit is not needed at all.
        _pending.remove(row.name);
      } else {
        _pending[row.name] = LockSetUnlockedEdit(
          lock: row.name,
          unlocked: unlocked,
        );
      }
      _pushPending();
    });
  }

  void _discardPending() {
    setState(() {
      _pending.clear();
      _pushPending();
    });
  }

  /// Every lock, catalog first and then the save-only names the catalog does
  /// not know, each carrying the state the save records for it.
  List<_LockRow> get _allRows {
    final catalog = _catalog;
    if (catalog == null) return const [];
    final rows = [
      for (final entry in catalog.locks)
        _LockRow(
          name: entry.name,
          entry: entry,
          unlocked: _saved.isUnlocked(entry.name),
        ),
    ];
    final known = {for (final entry in catalog.locks) entry.name.toLowerCase()};
    for (final name in _saved.unlocked) {
      if (known.contains(name.toLowerCase())) continue;
      rows.add(_LockRow(name: name, entry: null, unlocked: true));
    }
    return rows;
  }

  /// [withArea] false answers "would this row match if no region were
  /// selected?", which is what the rail's per-region counts need.
  bool _matchesFilters(_LockRow row, {bool withArea = true}) {
    if (withArea && _area.isNotEmpty && row.area != _area) return false;
    switch (_kind) {
      case _KindFilter.chests:
        if (row.isDoor) return false;
      case _KindFilter.doors:
        if (!row.isDoor) return false;
      case _KindFilter.all:
        break;
    }
    final effective = _pending[row.name]?.unlocked ?? row.unlocked;
    switch (_state) {
      case _StateFilter.unlocked:
        if (!effective) return false;
      case _StateFilter.locked:
        if (effective) return false;
      case _StateFilter.all:
        break;
    }
    if (_query.isEmpty) return true;
    return (_searchIndex[row.name] ?? row.search).contains(_query);
  }

  /// Searchable text per lock, including each key's LOCALIZED name.
  ///
  /// The catalog can only pre-compute the technical ids, but the row shows the
  /// game's own item name — so without this, typing what you see finds
  /// nothing while the field promises you can search by key. Rebuilt only when
  /// the localization behind those names changes, not per keystroke.
  void _refreshSearchIndex(
    Map<String, Map<String, String>> locCatalog,
    GameLang lang,
  ) {
    // The catalog is part of the key: the first build runs before it has
    // loaded, and keying on the localization alone cached that empty index
    // forever — the ids kept matching (they are in the catalog's own search
    // string) while every localized name silently did not.
    final key = (locCatalog, lang, _catalog);
    if (_searchIndexKey == key) return;
    _searchIndexKey = key;
    _searchIndex = {
      for (final entry in _catalog?.locks ?? const <LockEntry>[])
        if (entry.keys.isNotEmpty)
          entry.name: [
            entry.search,
            for (final id in entry.keys) localizedKeyName(locCatalog, lang, id),
          ].join(' ').toLowerCase(),
    };
  }

  @override
  Widget build(BuildContext context) {
    // Rebuild when a pending edit is dropped elsewhere (the global Save, or a
    // reset from another panel), so the row switches stop reflecting a draft
    // that no longer exists.
    ref.watch(editorProvider.select((state) => state.pendingEdits));
    _seedFromPending();
    final l10n = AppLocalizations.of(context);
    final scheme = widget.theme.colorScheme;
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final lang = ref.watch(currentGameLangProvider);
    final showObjectIds = ref.watch(showObjectIdsProvider);

    _refreshSearchIndex(locCatalog, lang);
    final rows = _allRows;
    final visible = rows.where(_matchesFilters).toList();
    // Region counts follow the kind/state/search filters but NOT the region
    // itself, so the rail keeps showing where the remaining matches are.
    final byArea = <String, int>{};
    var acrossRegions = 0;
    for (final row in rows) {
      if (!_matchesFilters(row, withArea: false)) continue;
      acrossRegions++;
      byArea[row.area] = (byArea[row.area] ?? 0) + 1;
    }

    final canEdit = widget.editable && _saved.canSetUnlocked && !_loading;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // No card title: the World sidebar tile already names the section.
            if (_saved.error != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  _saved.error!,
                  style: TextStyle(color: scheme.error),
                ),
              ),
            if (_saved.error == null && !_loading && !canEdit)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  l10n.locksReadOnly,
                  style: widget.theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ),
            Expanded(
              child: _loading && rows.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        SizedBox(
                          width: 220,
                          child: _RegionRail(
                            counts: byArea,
                            total: acrossRegions,
                            selected: _area,
                            locations: _locations,
                            locCatalog: locCatalog,
                            lang: lang,
                            l10n: l10n,
                            theme: widget.theme,
                            onSelected: (area) => setState(() => _area = area),
                          ),
                        ),
                        const SizedBox(width: 12),
                        const VerticalDivider(width: 1),
                        const SizedBox(width: 12),
                        Expanded(
                          child: _LockList(
                            rows: visible,
                            total: rows.length,
                            pending: _pending,
                            canEdit: canEdit,
                            query: _query,
                            kind: _kind,
                            state: _state,
                            locCatalog: locCatalog,
                            lang: lang,
                            showObjectIds: showObjectIds,
                            l10n: l10n,
                            theme: widget.theme,
                            onQuery: (value) => setState(
                              () => _query = value.trim().toLowerCase(),
                            ),
                            onKind: (value) => setState(() => _kind = value),
                            onState: (value) => setState(() => _state = value),
                            onToggle: _toggle,
                            onDiscard: _discardPending,
                          ),
                        ),
                      ],
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Left rail: one row per region, with the number of locks it holds under the
/// current kind/state/search filters.
class _RegionRail extends StatelessWidget {
  const _RegionRail({
    required this.counts,
    required this.total,
    required this.selected,
    required this.locations,
    required this.locCatalog,
    required this.lang,
    required this.l10n,
    required this.theme,
    required this.onSelected,
  });

  final Map<String, int> counts;
  final int total;
  final String selected;
  final LocationCatalog? locations;
  final Map<String, Map<String, String>> locCatalog;
  final GameLang lang;
  final AppLocalizations l10n;
  final ThemeData theme;
  final ValueChanged<String> onSelected;

  String _label(String area) {
    final catalog = locations;
    if (catalog == null) return area.isEmpty ? l10n.locationAreaOther : area;
    return localizedAreaLabel(area, catalog, locCatalog, lang, l10n);
  }

  @override
  Widget build(BuildContext context) {
    // Busiest region first: the Old Camp alone holds nearly half the locks, and
    // an alphabetical rail would bury it below a dozen one-lock corners.
    final areas = counts.keys.toList()
      ..sort((a, b) {
        final byCount = counts[b]!.compareTo(counts[a]!);
        return byCount != 0 ? byCount : _label(a).compareTo(_label(b));
      });

    return ListView(
      padding: EdgeInsets.zero,
      children: [
        _RegionTile(
          label: l10n.locksAllRegions,
          count: total,
          selected: selected.isEmpty,
          onTap: () => onSelected(''),
          theme: theme,
        ),
        for (final area in areas)
          _RegionTile(
            label: _label(area),
            count: counts[area]!,
            selected: selected == area,
            onTap: () => onSelected(area),
            theme: theme,
          ),
      ],
    );
  }
}

class _RegionTile extends StatelessWidget {
  const _RegionTile({
    required this.label,
    required this.count,
    required this.selected,
    required this.onTap,
    required this.theme,
  });

  final String label;
  final int count;
  final bool selected;
  final VoidCallback onTap;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      key: ValueKey('locks-region-$label'),
      dense: true,
      selected: selected,
      selectedTileColor: theme.colorScheme.primaryContainer,
      title: _EllipsisTooltip(text: label),
      trailing: Text(
        '$count',
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
      onTap: onTap,
    );
  }
}

/// Right pane: search, the two filter chip rows, and the rows themselves.
class _LockList extends StatelessWidget {
  const _LockList({
    required this.rows,
    required this.total,
    required this.pending,
    required this.canEdit,
    required this.query,
    required this.kind,
    required this.state,
    required this.locCatalog,
    required this.lang,
    required this.showObjectIds,
    required this.l10n,
    required this.theme,
    required this.onQuery,
    required this.onKind,
    required this.onState,
    required this.onToggle,
    required this.onDiscard,
  });

  final List<_LockRow> rows;
  final int total;
  final Map<String, LockSetUnlockedEdit> pending;
  final bool canEdit;
  final String query;
  final _KindFilter kind;
  final _StateFilter state;
  final Map<String, Map<String, String>> locCatalog;
  final GameLang lang;
  final bool showObjectIds;
  final AppLocalizations l10n;
  final ThemeData theme;
  final ValueChanged<String> onQuery;
  final ValueChanged<_KindFilter> onKind;
  final ValueChanged<_StateFilter> onState;
  final void Function(_LockRow row, bool unlocked) onToggle;
  final VoidCallback onDiscard;

  String _kindLabel(_KindFilter value) => switch (value) {
    _KindFilter.all => l10n.categoryAll,
    _KindFilter.chests => l10n.locksFilterChests,
    _KindFilter.doors => l10n.locksFilterDoors,
  };

  String _stateLabel(_StateFilter value) => switch (value) {
    _StateFilter.all => l10n.categoryAll,
    _StateFilter.unlocked => l10n.locksFilterUnlocked,
    _StateFilter.locked => l10n.locksFilterLocked,
  };

  String _keyName(String id) {
    final name = localizedKeyName(locCatalog, lang, id);
    return showObjectIds && name != id ? '$name ($id)' : name;
  }

  /// The line under a lock's name: what it takes to open it.
  String? _subtitleText(_LockRow row) {
    final entry = row.entry;
    if (entry == null) return l10n.locksUnknownEntry;
    // `Permalocked` is a sentinel key that matches no item in the game — the
    // lock is meant never to open, so say that instead of naming a key the
    // player can never hold.
    if (entry.keys.length == 1 && entry.keys.first == 'Permalocked') {
      return l10n.locksPermalocked;
    }
    final parts = <String>[];
    // A lock with no pickable difficulty at all only ever opens with its key.
    if (entry.difficulty == null && entry.keys.isNotEmpty) {
      parts.add(l10n.locksKeyOnly);
    }
    if (entry.keys.isNotEmpty) {
      parts.add(l10n.locksKeyLabel(entry.keys.map(_keyName).join(', ')));
    }
    return parts.isEmpty ? null : parts.join(' · ');
  }

  /// The line under a lock's name: the game's own difficulty pips, then what it
  /// takes to open it.
  Widget? _subtitle(_LockRow row) {
    final text = _subtitleText(row);
    final difficulty = row.entry?.difficulty;
    if (difficulty == null) return text == null ? null : Text(text);
    return Row(
      children: [
        _DifficultyBars(difficulty: difficulty, theme: theme, l10n: l10n),
        if (text != null) ...[
          const SizedBox(width: 8),
          Flexible(child: Text(text, overflow: TextOverflow.ellipsis)),
        ],
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = theme.colorScheme;
    final anyDoorVisible = rows.any((row) => row.isDoor);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: TextField(
                key: const Key('locks-search'),
                decoration: InputDecoration(
                  isDense: true,
                  prefixIcon: const Icon(Icons.search),
                  hintText: l10n.locksSearchHint,
                ),
                onChanged: onQuery,
              ),
            ),
            if (pending.isNotEmpty) ...[
              const SizedBox(width: 8),
              IconButton(
                key: const Key('locks-discard-pending'),
                icon: const Icon(Icons.undo_outlined),
                tooltip: l10n.locksResetPending,
                onPressed: onDiscard,
              ),
            ],
          ],
        ),
        const SizedBox(height: 8),
        // One row for both groups; the Wrap breaks to a second line only when
        // the pane is too narrow. The wider gap between the groups is what
        // keeps their two "All" chips from reading as one broken group.
        Wrap(
          spacing: 7,
          runSpacing: 6,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            for (final value in _KindFilter.values)
              ChoiceChip(
                key: ValueKey('locks-kind-${value.name}'),
                label: Text(_kindLabel(value)),
                selected: kind == value,
                onSelected: (_) => onKind(value),
              ),
            const SizedBox(width: 28),
            for (final value in _StateFilter.values)
              ChoiceChip(
                key: ValueKey('locks-state-${value.name}'),
                label: Text(_stateLabel(value)),
                selected: state == value,
                onSelected: (_) => onState(value),
              ),
            const SizedBox(width: 4),
            Text(
              l10n.locksShownOfTotal(rows.length, total),
              style: theme.textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        if (anyDoorVisible)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              l10n.locksDoorLeafHint,
              style: theme.textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ),
        const SizedBox(height: 12),
        Expanded(
          child: rows.isEmpty
              ? Center(
                  child: Text(
                    l10n.noEntriesMatch,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                )
              : ListView.separated(
                  key: const Key('locks-list'),
                  itemCount: rows.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, index) {
                    final row = rows[index];
                    final draft = pending[row.name];
                    final effective = draft?.unlocked ?? row.unlocked;
                    final subtitle = _subtitle(row);
                    return ListTile(
                      key: ValueKey('lock-${row.name}'),
                      dense: true,
                      // The game's own marks: the interaction glyph it draws on
                      // a container and on a door.
                      leading: GameIcon(
                        name: row.isDoor
                            ? 'T_Interaction_Door'
                            : 'T_Interaction_Loot',
                        fallbackIcon: row.isDoor
                            ? Icons.meeting_room_outlined
                            : Icons.inventory_2_outlined,
                        color: effective ? scheme.primary : scheme.outline,
                      ),
                      title: Text(row.name),
                      subtitle: draft != null
                          ? Text(l10n.glossaryPending)
                          : subtitle,
                      // The switch alone does not say which way is which, so
                      // the state is spelled out beside it — the word is what
                      // the row means, the switch is only how it is changed.
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            effective
                                ? l10n.locksFilterUnlocked
                                : l10n.locksFilterLocked,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: effective
                                  ? scheme.primary
                                  : scheme.onSurfaceVariant,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Switch.adaptive(
                            value: effective,
                            onChanged: canEdit
                                ? (value) => onToggle(row, value)
                                : null,
                          ),
                        ],
                      ),
                      onTap: canEdit ? () => onToggle(row, !effective) : null,
                    );
                  },
                ),
        ),
      ],
    );
  }
}

/// The game's own difficulty display: four pips, filled up to the lock's tier.
///
/// The lockpicking widget fills pip N when the tier reaches its threshold, and
/// the thresholds are 1, 2, 4, 6 — so a player never sees the raw 1..7 number,
/// only how many of four bars are lit. Showing the raw tier here would invent a
/// scale the game does not have, so it is offered as a tooltip instead.
class _DifficultyBars extends StatelessWidget {
  const _DifficultyBars({
    required this.difficulty,
    required this.theme,
    required this.l10n,
  });

  final int difficulty;
  final ThemeData theme;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final filled = lockDifficultyBars(difficulty);
    final scheme = theme.colorScheme;
    final message = l10n.locksDifficultyLevel(filled, difficulty);
    return Tooltip(
      message: message,
      child: Semantics(
        label: message,
        excludeSemantics: true,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            for (var bar = 1; bar <= lockDifficultyBarCount; bar++)
              Padding(
                padding: const EdgeInsets.only(right: 2),
                child: Container(
                  width: 4,
                  // Rising pips, like the game's own signal-strength shape.
                  height: 5.0 + bar * 2,
                  decoration: BoxDecoration(
                    color: bar <= filled
                        ? scheme.primary
                        : scheme.outlineVariant,
                    borderRadius: BorderRadius.circular(1),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// One line of text that ellipsises, and offers the full string as a tooltip
/// ONLY when it actually had to be cut.
///
/// A tooltip on every label would fire on the many that fit, which is noise;
/// measuring first means the hover only ever appears where it tells the reader
/// something they cannot already see. The region rail is where this bites —
/// "Illegale Sumpfkrautmischer" does not fit a 220px rail in any language.
class _EllipsisTooltip extends StatelessWidget {
  const _EllipsisTooltip({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final label = Text(text, maxLines: 1, overflow: TextOverflow.ellipsis);
    return LayoutBuilder(
      builder: (context, constraints) {
        if (!constraints.hasBoundedWidth) return label;
        // The enclosing DefaultTextStyle is the one the row actually paints
        // with (ListTile installs its own), so measuring against it matches
        // what the reader sees.
        final effective = DefaultTextStyle.of(context).style;
        final painter = TextPainter(
          text: TextSpan(text: text, style: effective),
          maxLines: 1,
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        final truncated = painter.didExceedMaxLines;
        painter.dispose();
        return truncated ? Tooltip(message: text, child: label) : label;
      },
    );
  }
}
