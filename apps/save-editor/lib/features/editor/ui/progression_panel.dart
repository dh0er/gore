import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/loc/progression_loc.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/knowledge_catalog.dart';
import '../domain/npc_catalog.dart';
import '../domain/pending_edits.dart';
import '../domain/progression_models.dart';
import 'add_knowledge_entry_dialog.dart';
import 'add_npc_dialog.dart';

/// All five EQuestState values, in dropdown order.
const questStates = <String>[
  'EQuestState::None',
  'EQuestState::Available',
  'EQuestState::Running',
  'EQuestState::Succeeded',
  'EQuestState::Failed',
];

/// Short state labels shown in filter chips, in display order.
const _filterStateLabels = <String>[
  'Available',
  'Running',
  'Succeeded',
  'Failed',
  'None',
];

String shortStateLabel(String state) {
  final idx = state.lastIndexOf('::');
  return idx < 0 ? state : state.substring(idx + 2);
}

/// Localized game name for a progression class/character id, or [fallback] when
/// the extracted loc catalog has no entry for it.
String _localizedProgressionName(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String id,
  String fallback,
) {
  return localizedGameName(catalog, lang, id) ?? fallback;
}

/// Client-side match for a character row: true when [query] (already trimmed
/// and lower-cased) is empty, or the raw [id] OR its localized display name
/// contains it. Lets the NPC/character search hit localized names, not just
/// the raw save id (which is all the core can filter on).
bool _characterMatches(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String id,
  String query,
) {
  if (query.isEmpty) return true;
  if (id.toLowerCase().contains(query)) return true;
  final name = localizedGameName(catalog, lang, id);
  return name != null && name.toLowerCase().contains(query);
}

/// Maps an English short state label to its localized form, defaulting to the
/// input itself for unknown values.
String _localizedShortLabel(AppLocalizations l10n, String label) {
  switch (label) {
    case 'None':
      return l10n.questStateNone;
    case 'Available':
      return l10n.questStateAvailable;
    case 'Running':
      return l10n.questStateRunning;
    case 'Succeeded':
      return l10n.questStateSucceeded;
    case 'Failed':
      return l10n.questStateFailed;
    default:
      return label;
  }
}

/// Localizes a raw EQuestState string (or null) for display. The 'unknown'
/// fallback maps to a dedicated localized label.
String _localizedState(AppLocalizations l10n, String? rawState) {
  final raw = rawState ?? 'unknown';
  final label = shortStateLabel(raw);
  if (label == 'unknown') return l10n.questStateUnknown;
  return _localizedShortLabel(l10n, label);
}

/// Sidebar section entries for the Progression tab.
enum _ProgSection { quests, knowledge, events, factions }

/// Progression tab: structured quests / dialog knowledge / memory events.
/// Full-height sidebar layout (no outer scroll). [reloadKey] is the
/// [SaveInspection] instance itself; identity comparison means every fresh
/// inspection clears local pending state and reloads.
class ProgressionPanel extends StatefulWidget {
  const ProgressionPanel({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  @override
  State<ProgressionPanel> createState() => _ProgressionPanelState();
}

class _ProgressionPanelState extends State<ProgressionPanel> {
  // Keep selected section across save-triggered reloads (identity comparison
  // on reloadKey, not path comparison, so same pattern as hero_stats_card).
  _ProgSection _selected = _ProgSection.quests;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (!widget.inspection.privateDecoded) {
      return _MessagePane(
        icon: Icons.flag_outlined,
        title: l10n.tabProgression,
        body: l10n.progressionLockedBody,
      );
    }
    if (!widget.inspection.privateProgression.available) {
      return _MessagePane(
        icon: Icons.flag_outlined,
        title: l10n.tabProgression,
        body: l10n.progressionNeedsTyped,
      );
    }

    final reloadKey = widget.inspection;
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Left sidebar: same style as the Player tab (hero_stats_card).
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
                    _SidebarTile(
                      icon: Icons.flag_outlined,
                      label: l10n.sectionQuests,
                      selected: _selected == _ProgSection.quests,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.quests),
                    ),
                    _SidebarTile(
                      icon: Icons.school_outlined,
                      label: l10n.sectionKnowledge,
                      selected: _selected == _ProgSection.knowledge,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.knowledge),
                    ),
                    _SidebarTile(
                      icon: Icons.history_outlined,
                      label: l10n.sectionEvents,
                      selected: _selected == _ProgSection.events,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.events),
                    ),
                    _SidebarTile(
                      icon: Icons.gavel_outlined,
                      label: l10n.factionsSidebar,
                      selected: _selected == _ProgSection.factions,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.factions),
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(width: 16),
          // Detail area — fills remaining width and full height.
          // Every section stays mounted (Offstage, same pattern as
          // hero_stats_card): a detail's local `_pending` map backs entries in
          // the global pending-edit registry, so disposing it on a section
          // switch would hide queued edits that Save still writes. Keys are
          // stable on purpose: a key derived from reloadKey would remount the
          // detail on every fresh inspection, disposing state and bypassing
          // the didUpdateWidget logic that preserves the selected character.
          Expanded(
            child: Stack(
              children: [
                Offstage(
                  offstage: _selected != _ProgSection.quests,
                  child: _QuestsDetail(
                    key: const ValueKey('quests'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
                Offstage(
                  offstage: _selected != _ProgSection.knowledge,
                  child: _KnowledgeDetail(
                    key: const ValueKey('knowledge'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
                Offstage(
                  offstage: _selected != _ProgSection.events,
                  child: _EventsDetail(
                    key: const ValueKey('events'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
                Offstage(
                  offstage: _selected != _ProgSection.factions,
                  child: _FactionsDetail(
                    key: const ValueKey('factions'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Sidebar tile
// ---------------------------------------------------------------------------

class _SidebarTile extends StatelessWidget {
  const _SidebarTile({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      child: Material(
        color: selected ? scheme.primaryContainer : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 18,
                  color: selected ? scheme.primary : scheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: selected ? scheme.primary : scheme.onSurface,
                      fontWeight: selected ? FontWeight.w600 : null,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Group picker tile (quests left pane)
// ---------------------------------------------------------------------------

class _GroupTile extends StatelessWidget {
  const _GroupTile({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return ListTile(
      dense: true,
      selected: selected,
      title: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
      selectedTileColor: scheme.primaryContainer,
      selectedColor: scheme.primary,
      onTap: onTap,
    );
  }
}

// ---------------------------------------------------------------------------
// Shared pagination bar widget
// ---------------------------------------------------------------------------

class _PaginationBar extends StatelessWidget {
  const _PaginationBar({
    required this.offset,
    required this.count,
    required this.total,
    required this.pageSize,
    required this.busy,
    required this.onPage,
    required this.onPageSize,
  });

  static const _pageSizes = [25, 50, 100, 250, 500];

  final int offset;
  final int count;
  final int total;
  final int pageSize;
  final bool busy;
  final void Function(int newOffset) onPage;
  final void Function(int newPageSize) onPageSize;

  int get _pageIndex => pageSize == 0 ? 0 : offset ~/ pageSize;
  int get _pageCount => total == 0 ? 1 : (total + pageSize - 1) ~/ pageSize;
  bool get _hasPrevious => offset > 0;
  bool get _hasNext => offset + count < total;

  @override
  Widget build(BuildContext context) {
    if (total == 0) return const SizedBox.shrink();
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final first = total == 0 ? 0 : offset + 1;
    final last = offset + count;
    final effectivePageSize = _pageSizes.contains(pageSize)
        ? pageSize
        : _pageSizes.reduce(
            (a, b) => (a - pageSize).abs() < (b - pageSize).abs() ? a : b,
          );
    return Wrap(
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 0,
      runSpacing: 4,
      children: [
        IconButton(
          tooltip: l10n.firstPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.first_page),
          onPressed: busy || !_hasPrevious ? null : () => onPage(0),
        ),
        IconButton(
          tooltip: l10n.previousPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_left),
          onPressed: busy || !_hasPrevious
              ? null
              : () => onPage((_pageIndex - 1) * pageSize),
        ),
        IconButton(
          tooltip: l10n.nextPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_right),
          onPressed: busy || !_hasNext
              ? null
              : () => onPage((_pageIndex + 1) * pageSize),
        ),
        IconButton(
          tooltip: l10n.lastPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.last_page),
          onPressed: busy || !_hasNext
              ? null
              : () => onPage((_pageCount - 1) * pageSize),
        ),
        const SizedBox(width: 4),
        Text(l10n.pageOfPages(_pageIndex + 1, _pageCount), style: muted),
        const SizedBox(width: 8),
        Text(l10n.rangeOfTotal(first, last, total), style: muted),
        const SizedBox(width: 8),
        Text(l10n.perPage, style: muted),
        const SizedBox(width: 6),
        DropdownButton<int>(
          value: effectivePageSize,
          isDense: true,
          underline: const SizedBox.shrink(),
          onChanged: busy ? null : (v) => v != null ? onPageSize(v) : null,
          items: [
            for (final sz in _pageSizes)
              DropdownMenuItem(value: sz, child: Text('$sz')),
          ],
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Quests detail (stateful, pending-edit pattern)
// ---------------------------------------------------------------------------

class _QuestsDetail extends ConsumerStatefulWidget {
  const _QuestsDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<_QuestsDetail> createState() => _QuestsDetailState();
}

class _QuestsDetailState extends ConsumerState<_QuestsDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _search = TextEditingController();
  // Full quest list (fetched once with a large limit, no server filters):
  // search, faceting and pagination are done client-side so the query can
  // match the localized quest name, not just the raw class_path the core sees.
  List<ProgressionQuest> _allQuests = const [];
  String? _fetchError;
  final Map<String, QuestStateChange> _pending = {};
  bool _loading = false;
  int _reloadEpoch = 0;
  int _pageSize = _defaultPageSize;
  // Client-side search/pagination state (query trimmed + lower-cased).
  String _query = '';
  int _offset = 0;
  String? _stateFilter;
  String? _groupFilter;
  // The core clamps a query's `limit` to 1000, so the full quest list must be
  // pulled page-by-page rather than in one oversized request.
  static const _fetchPageLimit = 1000;

  @override
  void initState() {
    super.initState();
    _loadAllQuests();
  }

  @override
  void didUpdateWidget(covariant _QuestsDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _search.clear();
      _query = '';
      _offset = 0;
      _stateFilter = null;
      _groupFilter = null;
      _loadAllQuests();
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  /// Fetches the full quest list. Filtering, faceted counts and pagination
  /// then happen client-side in [build].
  Future<void> _loadAllQuests() async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final all = <ProgressionQuest>[];
    String? error;
    var offset = 0;
    while (true) {
      final page = await widget.notifier.loadProgressionQuests(
        offset: offset,
        limit: _fetchPageLimit,
      );
      if (!mounted || epoch != _reloadEpoch) return;
      if (page.error != null) {
        error = page.error;
        break;
      }
      all.addAll(page.quests);
      offset += page.quests.length;
      // Stop on an empty page (defensive) or once the whole set is collected.
      if (page.quests.isEmpty || offset >= page.total) break;
    }
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      // Drop any partially-fetched pages on error so search/filter/edit never
      // operate on a silently-incomplete list; surface the error instead.
      _allQuests = error == null ? all : const [];
      _fetchError = error;
    });
  }

  void _applySearch() {
    setState(() {
      _query = _search.text.trim().toLowerCase();
      _offset = 0;
    });
  }

  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit('progression.quests');
    } else {
      widget.notifier.setPendingEdit(
        'progression.quests',
        PendingSaveEdit(
          edits: _pending.values.map((c) => c.toEditJson()).toList(),
        ),
      );
    }
  }

  void _setQuestState(ProgressionQuest quest, String? state) {
    setState(() {
      if (state == null || state == quest.currentState) {
        _pending.remove(quest.questClass);
      } else {
        _pending[quest.questClass] = QuestStateChange(
          statePath: quest.statePath,
          state: state,
        );
      }
    });
    _pushPending();
  }

  /// Short state label used for faceting, mirroring the core: a null state
  /// folds to 'unknown'.
  String _questLabel(ProgressionQuest e) =>
      e.currentState == null ? 'unknown' : shortStateLabel(e.currentState!);

  /// Builds the filtered + faceted + paginated quest view from [_allQuests].
  /// Faceting mirrors the core: stateCounts ignore the state filter and
  /// groupCounts ignore the group filter, while the query (id OR localized
  /// name) and the other filter apply to both.
  ProgressionQuestPage _computeView(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
  ) {
    final q = _query;
    bool matchesQuery(ProgressionQuest e) {
      if (q.isEmpty) return true;
      if (e.questClass.toLowerCase().contains(q)) return true;
      final name = localizedQuestName(catalog, lang, e.id);
      return name != null && name.toLowerCase().contains(q);
    }

    bool matchesState(ProgressionQuest e) {
      final sf = _stateFilter;
      if (sf == null) return true;
      final lf = sf.toLowerCase();
      return _questLabel(e).toLowerCase() == lf ||
          (e.currentState?.toLowerCase() ?? '') == lf;
    }

    bool matchesGroup(ProgressionQuest e) {
      final gf = _groupFilter;
      return gf == null || e.group.toLowerCase() == gf.toLowerCase();
    }

    final stateCounts = <String, int>{};
    final groupCounts = <String, int>{};
    for (final e in _allQuests) {
      if (!matchesQuery(e)) continue;
      if (matchesGroup(e)) {
        final l = _questLabel(e);
        stateCounts[l] = (stateCounts[l] ?? 0) + 1;
      }
      if (matchesState(e)) {
        groupCounts[e.group] = (groupCounts[e.group] ?? 0) + 1;
      }
    }

    final filtered =
        _allQuests
            .where(
              (e) => matchesQuery(e) && matchesState(e) && matchesGroup(e),
            )
            .toList()
          ..sort((a, b) => a.questClass.compareTo(b.questClass));
    final total = filtered.length;
    final offset = _offset < total ? _offset : 0;
    final pageQuests = filtered.skip(offset).take(_pageSize).toList();
    return ProgressionQuestPage(
      quests: pageQuests,
      stateCounts: stateCounts,
      groupCounts: groupCounts,
      total: total,
      offset: offset,
      limit: _pageSize,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};
    final scheme = widget.theme.colorScheme;
    final page = _computeView(locCatalog, lang);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        // Two-pane row: group picker left, quest content right.
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Left pane: scrollable group picker.
            SizedBox(width: 190, child: _buildGroupPicker(l10n, page)),
            const SizedBox(width: 12),
            const VerticalDivider(width: 1),
            const SizedBox(width: 12),
            // Right pane: existing header + search + status chips + list.
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Header row
                  Row(
                    children: [
                      Icon(Icons.flag_outlined, color: scheme.primary),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          l10n.sectionQuests,
                          style: widget.theme.textTheme.titleMedium,
                        ),
                      ),
                      if (widget.editable && _pending.isNotEmpty)
                        Tooltip(
                          message: l10n.resetQuestChanges,
                          child: IconButton(
                            icon: const Icon(Icons.undo_outlined),
                            onPressed: () {
                              setState(_pending.clear);
                              widget.notifier.clearPendingEdit(
                                'progression.quests',
                              );
                            },
                          ),
                        ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  // Search field — client-side, filters live as you type.
                  TextField(
                    controller: _search,
                    decoration: InputDecoration(
                      labelText: l10n.searchQuests,
                      prefixIcon: const Icon(Icons.search),
                      suffixIcon: IconButton(
                        icon: const Icon(Icons.arrow_forward),
                        onPressed: _applySearch,
                      ),
                    ),
                    onChanged: (_) => _applySearch(),
                    onSubmitted: (_) => _applySearch(),
                  ),
                  if (_fetchError != null) ...[
                    const SizedBox(height: 8),
                    Text(_fetchError!, style: TextStyle(color: scheme.error)),
                  ],
                  const SizedBox(height: 8),
                  // Filter row: status chips only (group selection lives in the
                  // left pane now).
                  _QuestFilterRow(
                    page: page,
                    stateFilter: _stateFilter,
                    busy: _loading,
                    onStateChanged: (label) {
                      setState(() {
                        _stateFilter = (_stateFilter == label) ? null : label;
                        _offset = 0;
                      });
                    },
                  ),
                  const SizedBox(height: 4),
                  _PaginationBar(
                    offset: page.offset,
                    count: page.quests.length,
                    total: page.total,
                    pageSize: _pageSize,
                    busy: _loading,
                    onPage: (o) => setState(() => _offset = o),
                    onPageSize: (s) => setState(() {
                      _pageSize = s;
                      _offset = 0;
                    }),
                  ),
                  const SizedBox(height: 4),
                  // Quest list — the only scrollable, fills remaining height
                  Expanded(
                    child: _loading && page.quests.isEmpty
                        ? const Center(child: CircularProgressIndicator())
                        : ListView.separated(
                            itemCount: page.quests.length,
                            separatorBuilder: (_, _) =>
                                const Divider(height: 1),
                            itemBuilder: (context, index) {
                              final quest = page.quests[index];
                              final pendingState =
                                  _pending[quest.questClass]?.state;
                              final effectiveState =
                                  pendingState ?? quest.currentState;
                              final inKnownStates =
                                  effectiveState != null &&
                                  questStates.contains(effectiveState);
                              return ListTile(
                                dense: true,
                                leading: const Icon(Icons.flag_outlined),
                                title: SelectableText(
                                  localizedQuestName(
                                        locCatalog,
                                        lang,
                                        quest.id,
                                      ) ??
                                      (quest.name.isEmpty
                                          ? quest.id
                                          : quest.name),
                                  maxLines: 1,
                                ),
                                subtitle: SelectableText(
                                  '${quest.group} / ${quest.id}',
                                  maxLines: 1,
                                ),
                                trailing:
                                    widget.editable &&
                                        quest.writable &&
                                        inKnownStates
                                    ? DropdownButton<String>(
                                        value: effectiveState,
                                        underline: const SizedBox.shrink(),
                                        items: questStates
                                            .map(
                                              (s) => DropdownMenuItem(
                                                value: s,
                                                child: Text(
                                                  _localizedState(l10n, s),
                                                ),
                                              ),
                                            )
                                            .toList(),
                                        onChanged: (s) =>
                                            _setQuestState(quest, s),
                                      )
                                    : Text(
                                        _localizedState(
                                          l10n,
                                          quest.currentState,
                                        ),
                                      ),
                              );
                            },
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

  /// Left-pane group picker: "All groups" entry plus one tile per group from
  /// [page.groupCounts], sorted alphabetically. The selected group is kept
  /// visible (count 0) even if it drops out of the latest counts. Tapping sets
  /// [_groupFilter] and re-filters client-side.
  Widget _buildGroupPicker(AppLocalizations l10n, ProgressionQuestPage page) {
    final sortedGroups = page.groupCounts.keys.toList()..sort();
    // Keep the selected group present even when its count is now 0.
    final selected = _groupFilter;
    if (selected != null && !page.groupCounts.containsKey(selected)) {
      sortedGroups.add(selected);
    }
    return ListView(
      padding: EdgeInsets.zero,
      children: [
        _GroupTile(
          label: l10n.allGroups,
          selected: selected == null,
          onTap: () {
            if (selected == null) return;
            setState(() {
              _groupFilter = null;
              _offset = 0;
            });
          },
        ),
        for (final g in sortedGroups)
          _GroupTile(
            label: l10n.groupWithCount(g, page.groupCounts[g] ?? 0),
            selected: selected == g,
            onTap: () {
              if (selected == g) return;
              setState(() {
                _groupFilter = g;
                _offset = 0;
              });
            },
          ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Knowledge detail — two-pane: NPC list left, entries right
// ---------------------------------------------------------------------------

class _KnowledgeDetail extends ConsumerStatefulWidget {
  const _KnowledgeDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<_KnowledgeDetail> createState() => _KnowledgeDetailState();
}

class _KnowledgeDetailState extends ConsumerState<_KnowledgeDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _characterSearch = TextEditingController();
  // Holds the FULL character list (fetched with a large limit, no server
  // query): search and pagination are done client-side so the query can also
  // match localized display names, which only exist on the Dart side.
  KnowledgeCharactersPage _characters = const KnowledgeCharactersPage();
  String? _selectedCharacter;
  KnowledgeEntriesPage _entries = const KnowledgeEntriesPage();
  final Map<String, KnowledgeEntryEdit> _pending = {};
  final TextEditingController _addController = TextEditingController();
  bool _loadingCharacters = false;
  bool _loadingEntries = false;
  // Per-loader epochs so a stale characters load never races an entries load.
  int _charsEpoch = 0;
  int _entriesEpoch = 0;
  int _charPageSize = _defaultPageSize;
  int _entryPageSize = _defaultPageSize;
  // Client-side search/pagination state for the NPC list (trimmed+lower-cased
  // query, current page offset into the filtered list).
  String _charQuery = '';
  int _charOffset = 0;
  // The core clamps a query's `limit` to 1000, so the full NPC list must be
  // pulled page-by-page rather than in one oversized request.
  static const _fetchPageLimit = 1000;
  // Used during the cross-page duplicate check in _addEntry.
  bool _checkingDuplicate = false;
  // Error text shown beneath the add field (duplicate-check failure, etc.).
  String? _addError;
  // Bundled catalogs for the NPC + knowledge-entry picker dialogs. Null until
  // loaded asynchronously in initState.
  NpcCatalog? _npcCatalog;
  KnowledgeCatalog? _knowledgeCatalog;

  @override
  void initState() {
    super.initState();
    _loadCharacters();
    _loadCatalogs();
  }

  Future<void> _loadCatalogs() async {
    final npc = await NpcCatalog.loadBundled();
    final knowledge = await KnowledgeCatalog.loadBundled();
    if (!mounted) return;
    setState(() {
      _npcCatalog = npc;
      _knowledgeCatalog = knowledge;
    });
  }

  Future<void> _addNpc() async {
    final catalog = _npcCatalog;
    if (catalog == null) return;
    // Best-effort exclude: only NPCs on the currently loaded page are known
    // here (the list is paginated/searchable). The core rejects true
    // duplicates regardless.
    final existing = _characters.characters
        .map((c) => c.name.toLowerCase())
        .toSet();
    final picked = await showAddNpcDialog(
      context,
      catalog: catalog,
      exclude: existing,
    );
    if (picked == null || !mounted) return;
    final ok = await widget.notifier.applyAddKnowledgeCharacter(picked);
    if (!mounted || !ok) return; // on failure the notifier set state.error.
    await _loadCharacters();
    if (!mounted) return;
    await _selectCharacter(picked);
  }

  Future<void> _browseAddEntry() async {
    final catalog = _knowledgeCatalog;
    if (catalog == null) return;
    final existing = _entries.entries.map((e) => e.toLowerCase()).toSet();
    final picked = await showAddKnowledgeEntryDialog(
      context,
      catalog: catalog,
      exclude: existing,
    );
    if (picked == null || !mounted) return;
    await _addEntry(picked); // existing path: dup-check + pending edit.
  }

  @override
  void didUpdateWidget(covariant _KnowledgeDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      // Pending edits belong to the old inspection — always clear them.
      _pending.clear();
      // Invalidate both loaders so any in-flight call for the old reloadKey
      // is treated as stale and exits without touching flags.
      _charsEpoch++;
      _entriesEpoch++;
      _characterSearch.clear();
      _charQuery = '';
      _charOffset = 0;
      // A different save file means a different character set: drop the
      // selection. A same-file refresh (post-save reload) preserves it.
      if (widget.reloadKey.path != oldWidget.reloadKey.path) {
        _selectedCharacter = null;
      }
      _loadCharacters();
      if (_selectedCharacter != null) {
        _selectCharacter(_selectedCharacter!);
      } else {
        _entries = const KnowledgeEntriesPage();
      }
    }
  }

  @override
  void dispose() {
    _characterSearch.dispose();
    _addController.dispose();
    super.dispose();
  }

  /// Fetches the full knowledge-character list. Searching and pagination then
  /// happen client-side in [build] so the query can match localized names too.
  Future<void> _loadCharacters() async {
    final epoch = ++_charsEpoch;
    setState(() => _loadingCharacters = true);
    final all = <KnowledgeCharacter>[];
    KnowledgeCharactersPage? last;
    var offset = 0;
    while (true) {
      final page = await widget.notifier.loadKnowledgeCharacters(
        offset: offset,
        limit: _fetchPageLimit,
      );
      if (!mounted || epoch != _charsEpoch) return;
      last = page;
      if (page.error != null) break;
      all.addAll(page.characters);
      offset += page.characters.length;
      // Stop on an empty page (defensive) or once the whole set is collected.
      if (page.characters.isEmpty || offset >= page.total) break;
    }
    if (!mounted || epoch != _charsEpoch) return;
    // The loop always runs at least once, so `last` is non-null here. Read its
    // fields before setState so the closure doesn't depend on flow promotion.
    final error = last.error;
    final total = last.total;
    final limit = last.limit;
    setState(() {
      _loadingCharacters = false;
      // Drop any partially-fetched pages on error so search/filter/edit never
      // operate on a silently-incomplete list; surface the error instead.
      _characters = KnowledgeCharactersPage(
        characters: error == null ? all : const [],
        total: error == null ? total : 0,
        offset: 0,
        limit: limit,
        error: error,
      );
    });
  }

  void _applyCharSearch() {
    setState(() {
      _charQuery = _characterSearch.text.trim().toLowerCase();
      _charOffset = 0;
    });
  }

  Future<void> _selectCharacter(String name) async {
    final epoch = ++_entriesEpoch;
    setState(() {
      _selectedCharacter = name;
      _loadingEntries = true;
      _entries = const KnowledgeEntriesPage();
      _addError = null;
    });
    final page = await widget.notifier.loadKnowledgeEntries(
      name,
      offset: 0,
      limit: _entryPageSize,
    );
    if (!mounted || epoch != _entriesEpoch) return;
    setState(() {
      _loadingEntries = false;
      _entries = page;
    });
  }

  Future<void> _loadEntries({required int offset}) async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_entriesEpoch;
    setState(() => _loadingEntries = true);
    final page = await widget.notifier.loadKnowledgeEntries(
      character,
      offset: offset,
      limit: _entryPageSize,
    );
    if (!mounted || epoch != _entriesEpoch) return;
    setState(() {
      _loadingEntries = false;
      _entries = page;
    });
  }

  void _setEntryPageSize(int size) {
    setState(() => _entryPageSize = size);
    _loadEntries(offset: 0);
  }

  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit('progression.knowledge');
    } else {
      widget.notifier.setPendingEdit(
        'progression.knowledge',
        PendingSaveEdit(
          edits: _pending.values.map((e) => e.toEditJson()).toList(),
        ),
      );
    }
  }

  /// UE Names compare case-insensitively, so the entry part of the pending
  /// key is folded to lower case: an add and a remove that differ only in
  /// casing address the same logical entry and toggle instead of coexisting
  /// as conflicting setAdd + setRemove ops. The queued edit itself keeps the
  /// caller's original casing.
  String _pendingKey(String character, String entry) =>
      '$character\t${entry.toLowerCase()}';

  void _removeEntry(String entry) {
    // Same guard as _addEntry: never queue an edit with an unloaded path.
    if (_entries.setPath.isEmpty) return;
    final key = _pendingKey(_selectedCharacter!, entry);
    setState(() {
      if (_pending.containsKey(key)) {
        _pending.remove(key);
      } else {
        _pending[key] = KnowledgeEntryEdit.remove(
          setPath: _entries.setPath,
          entry: entry,
        );
      }
    });
    _pushPending();
  }

  Future<void> _addEntry(String entry) async {
    final l10n = AppLocalizations.of(context);
    // Issue C: defense-in-depth guard — setPath not yet loaded → reject.
    if (_entries.setPath.isEmpty) return;
    final trimmed = entry.trim();
    if (trimmed.isEmpty) return;
    // Fast path: already on the current page.
    // UE Names compare case-insensitively, so case-variants are duplicates.
    final trimmedLower = trimmed.toLowerCase();
    if (_entries.entries.any((e) => e.toLowerCase() == trimmedLower)) {
      setState(() => _addError = l10n.alreadyExistsForCharacter);
      return;
    }
    final character = _selectedCharacter!;
    // _pendingKey folds the entry to lower case, so this single lookup is
    // already case-insensitive.
    final key = _pendingKey(character, trimmed);
    if (_pending.containsKey(key)) {
      setState(() => _addError = l10n.alreadyInPendingChanges);
      return;
    }

    // Issue B: cross-page duplicate check via a server query.
    final checkCharacter = character;
    final checkEpoch = _entriesEpoch;
    setState(() {
      _checkingDuplicate = true;
      _addError = null;
    });
    // The core query is a lowercase-contains filter, so an exact match can
    // sit on any page of the match set — page through ALL matches.
    var exists = false;
    String? checkError;
    try {
      var offset = 0;
      while (true) {
        final checkPage = await widget.notifier.loadKnowledgeEntries(
          checkCharacter,
          query: trimmed,
          limit: 200,
          offset: offset,
        );
        if (!mounted ||
            _selectedCharacter != checkCharacter ||
            _entriesEpoch != checkEpoch) {
          return;
        }
        if (checkPage.error != null) {
          checkError = checkPage.error;
          break;
        }
        if (checkPage.entries.any((e) => e.toLowerCase() == trimmedLower)) {
          exists = true;
          break;
        }
        offset += checkPage.entries.length;
        if (checkPage.entries.isEmpty || offset >= checkPage.total) break;
      }
    } finally {
      // Clear the lock on every exit path (unmount, stale, error).
      if (mounted) setState(() => _checkingDuplicate = false);
    }
    if (!mounted ||
        _selectedCharacter != checkCharacter ||
        _entriesEpoch != checkEpoch) {
      return;
    }
    // A failed query must NOT fall through to a pending add.
    if (checkError != null) {
      setState(() {
        _addError = l10n.duplicateCheckFailed(checkError ?? '');
      });
      return;
    }
    if (exists) {
      setState(() => _addError = l10n.alreadyExistsForCharacter);
      return;
    }

    setState(() {
      _pending[key] = KnowledgeEntryEdit.add(
        setPath: _entries.setPath,
        entry: trimmed,
      );
      _addController.clear();
      _addError = null;
    });
    _pushPending();
  }

  void _undoAdd(String entry) {
    final key = _pendingKey(_selectedCharacter!, entry);
    setState(() => _pending.remove(key));
    _pushPending();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};
    final scheme = widget.theme.colorScheme;
    final character = _selectedCharacter;

    // Compute sets for rendering — only include edits for the selected NPC.
    // removedEntries holds lower-cased values to match the case-folded
    // pending keys: lookups against displayed entries fold too.
    final removedEntries = <String>{};
    final addedEntries = <String>[];
    if (character != null) {
      final prefix = '$character\t';
      for (final e in _pending.entries) {
        if (!e.key.startsWith(prefix)) continue;
        if (!e.value.isAdd) removedEntries.add(e.value.entry.toLowerCase());
        if (e.value.isAdd) addedEntries.add(e.value.entry);
      }
    }

    // Client-side search (id OR localized name) + pagination over the full
    // fetched character list.
    final filteredChars = _characters.characters
        .where((c) => _characterMatches(locCatalog, lang, c.name, _charQuery))
        .toList();
    final charTotal = filteredChars.length;
    final charOffset = _charOffset < charTotal ? _charOffset : 0;
    final pageChars = filteredChars
        .skip(charOffset)
        .take(_charPageSize)
        .toList();

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.school_outlined, color: scheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.dialogKnowledge,
                    style: widget.theme.textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && _pending.isNotEmpty)
                  Tooltip(
                    message: l10n.resetKnowledgeChanges,
                    child: IconButton(
                      icon: const Icon(Icons.undo_outlined),
                      onPressed: () {
                        setState(_pending.clear);
                        widget.notifier.clearPendingEdit(
                          'progression.knowledge',
                        );
                      },
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            // Two-pane row: characters left, entries right
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Left pane: NPC search + pagination + list
                  SizedBox(
                    width: 280,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        if (widget.editable) ...[
                          Align(
                            alignment: Alignment.centerLeft,
                            child: OutlinedButton.icon(
                              icon: const Icon(Icons.person_add_alt_1, size: 18),
                              label: Text(l10n.addNpc),
                              onPressed: _npcCatalog == null ? null : _addNpc,
                            ),
                          ),
                          const SizedBox(height: 8),
                        ],
                        TextField(
                          controller: _characterSearch,
                          decoration: InputDecoration(
                            labelText: l10n.searchNpcs,
                            isDense: true,
                            prefixIcon: const Icon(Icons.search, size: 18),
                            suffixIcon: IconButton(
                              icon: const Icon(Icons.arrow_forward, size: 18),
                              onPressed: _applyCharSearch,
                            ),
                          ),
                          // Client-side filter: apply live as the user types.
                          onChanged: (_) => _applyCharSearch(),
                          onSubmitted: (_) => _applyCharSearch(),
                        ),
                        if (_characters.error != null) ...[
                          const SizedBox(height: 4),
                          Text(
                            _characters.error!,
                            style: TextStyle(color: scheme.error, fontSize: 12),
                          ),
                        ],
                        const SizedBox(height: 4),
                        _PaginationBar(
                          offset: charOffset,
                          count: pageChars.length,
                          total: charTotal,
                          pageSize: _charPageSize,
                          busy: _loadingCharacters,
                          onPage: (o) => setState(() => _charOffset = o),
                          onPageSize: (s) => setState(() {
                            _charPageSize = s;
                            _charOffset = 0;
                          }),
                        ),
                        const SizedBox(height: 4),
                        Expanded(
                          child:
                              _loadingCharacters &&
                                  _characters.characters.isEmpty
                              ? const Center(child: CircularProgressIndicator())
                              : ListView.separated(
                                  itemCount: pageChars.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (context, index) {
                                    final c = pageChars[index];
                                    final isSelected = c.name == character;
                                    final displayName =
                                        _localizedProgressionName(
                                          locCatalog,
                                          lang,
                                          c.name,
                                          c.name,
                                        );
                                    return ListTile(
                                      dense: true,
                                      selected: isSelected,
                                      title: Text(
                                        l10n.characterWithCount(
                                          displayName,
                                          c.entryCount,
                                        ),
                                      ),
                                      // Show the raw save id beneath the name
                                      // when it differs from the localized
                                      // display name.
                                      subtitle: displayName == c.name
                                          ? null
                                          : Text(
                                              c.name,
                                              maxLines: 1,
                                              overflow: TextOverflow.ellipsis,
                                              style: TextStyle(
                                                color: scheme.onSurfaceVariant,
                                                fontSize: 11,
                                              ),
                                            ),
                                      onTap: () => _selectCharacter(c.name),
                                    );
                                  },
                                ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 12),
                  const VerticalDivider(width: 1),
                  const SizedBox(width: 12),
                  // Right pane: header + add field + pagination + entries list
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          character != null
                              ? l10n.entriesForCharacter(
                                  _localizedProgressionName(
                                    locCatalog,
                                    lang,
                                    character,
                                    character,
                                  ),
                                )
                              : l10n.selectNpcToSeeEntries,
                          style: widget.theme.textTheme.labelLarge,
                        ),
                        if (character != null) ...[
                          const SizedBox(height: 6),
                          if (widget.editable) ...[
                            // Issue C: disabled while entries are loading or
                            // the set path is not yet known.
                            Builder(
                              builder: (context) {
                                final addDisabled =
                                    _loadingEntries ||
                                    _entries.setPath.isEmpty ||
                                    _checkingDuplicate;
                                return Row(
                                  children: [
                                    Expanded(
                                      child: TextField(
                                        controller: _addController,
                                        enabled: !addDisabled,
                                        decoration: InputDecoration(
                                          labelText: l10n.addKnowledgeEntry,
                                          isDense: true,
                                        ),
                                        onSubmitted: addDisabled
                                            ? null
                                            : (v) => _addEntry(v),
                                      ),
                                    ),
                                    const SizedBox(width: 8),
                                    // Issue B: spinner during cross-page check.
                                    _checkingDuplicate
                                        ? const Padding(
                                            padding: EdgeInsets.all(12),
                                            child: SizedBox(
                                              width: 16,
                                              height: 16,
                                              child: CircularProgressIndicator(
                                                strokeWidth: 2,
                                              ),
                                            ),
                                          )
                                        : IconButton(
                                            icon: const Icon(Icons.add),
                                            tooltip: l10n.add,
                                            onPressed: addDisabled
                                                ? null
                                                : () => _addEntry(
                                                    _addController.text,
                                                  ),
                                          ),
                                    // Browse the full knowledge-entry catalog.
                                    // The free-text field above stays as a
                                    // fallback for non-catalog tokens.
                                    IconButton(
                                      icon: const Icon(Icons.menu_book_outlined),
                                      tooltip: l10n.browseCatalog,
                                      onPressed:
                                          addDisabled || _knowledgeCatalog == null
                                          ? null
                                          : _browseAddEntry,
                                    ),
                                  ],
                                );
                              },
                            ),
                          ],
                          if (_entries.error != null)
                            Padding(
                              padding: const EdgeInsets.only(top: 4),
                              child: Text(
                                _entries.error!,
                                style: TextStyle(color: scheme.error),
                              ),
                            ),
                          if (_addError != null)
                            Padding(
                              padding: const EdgeInsets.only(top: 4),
                              child: Text(
                                _addError!,
                                style: TextStyle(color: scheme.error),
                              ),
                            ),
                          const SizedBox(height: 4),
                          _PaginationBar(
                            offset: _entries.offset,
                            count: _entries.entries.length,
                            total: _entries.total,
                            pageSize: _entryPageSize,
                            busy: _loadingEntries,
                            onPage: (o) => _loadEntries(offset: o),
                            onPageSize: _setEntryPageSize,
                          ),
                          const SizedBox(height: 4),
                          // Issue E: pending adds in a separate labeled block
                          // so the pagination bar unambiguously refers to the
                          // saved entries below.
                          if (addedEntries.isNotEmpty) ...[
                            Padding(
                              padding: const EdgeInsets.symmetric(vertical: 4),
                              child: Text(
                                l10n.pendingAddsCount(addedEntries.length),
                                style: widget.theme.textTheme.labelSmall
                                    ?.copyWith(color: scheme.onSurfaceVariant),
                              ),
                            ),
                            for (final entry in addedEntries)
                              Builder(
                                builder: (context) {
                                  final text = localizedKnowledgeEntry(
                                    locCatalog,
                                    lang,
                                    entry,
                                  );
                                  return ListTile(
                                    dense: true,
                                    tileColor: scheme.tertiaryContainer
                                        .withValues(alpha: 0.4),
                                    title: Text(
                                      text ?? entry,
                                      style: TextStyle(
                                        color: scheme.onTertiaryContainer,
                                      ),
                                    ),
                                    subtitle: text == null
                                        ? null
                                        : Text(
                                            entry,
                                            maxLines: 1,
                                            overflow: TextOverflow.ellipsis,
                                            style: TextStyle(
                                              color: scheme.onSurfaceVariant,
                                              fontSize: 11,
                                            ),
                                          ),
                                    trailing: widget.editable
                                        ? IconButton(
                                            icon: const Icon(
                                              Icons.undo,
                                              size: 18,
                                            ),
                                            tooltip: l10n.undoAdd,
                                            onPressed: () => _undoAdd(entry),
                                          )
                                        : null,
                                  );
                                },
                              ),
                            const Divider(height: 8),
                          ],
                          // Saved entries list — the only scrollable in this
                          // pane; pagination bar above refers only to this.
                          Expanded(
                            child: _loadingEntries && _entries.entries.isEmpty
                                ? const Center(
                                    child: CircularProgressIndicator(),
                                  )
                                : ListView.separated(
                                    itemCount: _entries.entries.length,
                                    separatorBuilder: (_, _) =>
                                        const Divider(height: 1),
                                    itemBuilder: (context, index) {
                                      final entry = _entries.entries[index];
                                      final isRemoved = removedEntries.contains(
                                        entry.toLowerCase(),
                                      );
                                      final text = localizedKnowledgeEntry(
                                        locCatalog,
                                        lang,
                                        entry,
                                      );
                                      return ListTile(
                                        dense: true,
                                        title: Text(
                                          text ?? entry,
                                          style: isRemoved
                                              ? const TextStyle(
                                                  decoration: TextDecoration
                                                      .lineThrough,
                                                )
                                              : null,
                                        ),
                                        // Raw entry id beneath the resolved
                                        // dialog line, when one was found.
                                        subtitle: text == null
                                            ? null
                                            : Text(
                                                entry,
                                                maxLines: 1,
                                                overflow: TextOverflow.ellipsis,
                                                style: TextStyle(
                                                  color:
                                                      scheme.onSurfaceVariant,
                                                  fontSize: 11,
                                                  decoration: isRemoved
                                                      ? TextDecoration
                                                            .lineThrough
                                                      : null,
                                                ),
                                              ),
                                        trailing: widget.editable
                                            ? IconButton(
                                                icon: Icon(
                                                  isRemoved
                                                      ? Icons.undo
                                                      : Icons.delete_outline,
                                                  size: 18,
                                                ),
                                                tooltip: isRemoved
                                                    ? l10n.undoRemove
                                                    : l10n.removeEntry,
                                                onPressed: () =>
                                                    _removeEntry(entry),
                                              )
                                            : null,
                                      );
                                    },
                                  ),
                          ),
                        ],
                        if (character == null)
                          Expanded(
                            child: Center(
                              child: Text(
                                l10n.selectNpcFromList,
                                style: TextStyle(
                                  color: scheme.onSurfaceVariant,
                                ),
                              ),
                            ),
                          ),
                      ],
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

// ---------------------------------------------------------------------------
// Events detail — two-pane: character list left, events right
// ---------------------------------------------------------------------------

class _EventsDetail extends ConsumerStatefulWidget {
  const _EventsDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<_EventsDetail> createState() => _EventsDetailState();
}

class _EventsDetailState extends ConsumerState<_EventsDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _characterSearch = TextEditingController();
  // Memory-event characters use ids that have no loc entries (spawn-point /
  // internal names), so their search stays server-side: the set can exceed
  // the core's per-page cap, and there is no localized name to match anyway.
  MemoryCharactersPage _characters = const MemoryCharactersPage();
  String? _selectedCharacter;
  MemoryEventsPage _events = const MemoryEventsPage();
  bool _loadingCharacters = false;
  bool _searchingCharacters = false;
  bool _loadingEvents = false;
  // Per-loader epochs so a stale characters load never races an events load.
  int _charsEpoch = 0;
  int _eventsEpoch = 0;
  int _charPageSize = _defaultPageSize;
  int _eventPageSize = _defaultPageSize;
  String _activeCharQuery = '';

  @override
  void initState() {
    super.initState();
    _loadCharacters(offset: 0);
  }

  @override
  void didUpdateWidget(covariant _EventsDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      // Invalidate both loaders so any in-flight call for the old reloadKey
      // is treated as stale and exits without touching flags.
      _charsEpoch++;
      _eventsEpoch++;
      _characterSearch.clear();
      _activeCharQuery = '';
      // A different save file means a different character set: drop the
      // selection. A same-file refresh (post-event-edit reload) preserves it
      // so the right pane does not fall back to "Select a character".
      if (widget.reloadKey.path != oldWidget.reloadKey.path) {
        _selectedCharacter = null;
      }
      _loadCharacters(offset: 0);
      // Re-trigger the preserved character's events load (page reset to 0 is
      // fine). If it has since been deleted the existing error rendering
      // will handle it.
      if (_selectedCharacter != null) {
        _selectCharacter(_selectedCharacter!);
      } else {
        _events = const MemoryEventsPage();
      }
    }
  }

  @override
  void dispose() {
    _characterSearch.dispose();
    super.dispose();
  }

  Future<void> _loadCharacters({
    required int offset,
    bool newQuery = false,
  }) async {
    if (newQuery) _activeCharQuery = _characterSearch.text.trim();
    final epoch = ++_charsEpoch;
    setState(() {
      _loadingCharacters = true;
      if (newQuery) _searchingCharacters = true;
    });
    final page = await widget.notifier.loadMemoryCharacters(
      query: _activeCharQuery,
      offset: offset,
      limit: _charPageSize,
    );
    if (!mounted || epoch != _charsEpoch) return;
    setState(() {
      _loadingCharacters = false;
      _searchingCharacters = false;
      _characters = page;
    });
  }

  Future<void> _selectCharacter(String id) async {
    final epoch = ++_eventsEpoch;
    setState(() {
      _selectedCharacter = id;
      _loadingEvents = true;
      _events = const MemoryEventsPage(); // clear stale page immediately
    });
    final page = await widget.notifier.loadMemoryEvents(
      id,
      offset: 0,
      limit: _eventPageSize,
    );
    if (!mounted || epoch != _eventsEpoch) return;
    setState(() {
      _loadingEvents = false;
      _events = page;
    });
  }

  Future<void> _loadEvents({required int offset}) async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_eventsEpoch;
    setState(() => _loadingEvents = true);
    final page = await widget.notifier.loadMemoryEvents(
      character,
      offset: offset,
      limit: _eventPageSize,
    );
    if (!mounted || epoch != _eventsEpoch) return;
    setState(() {
      _loadingEvents = false;
      _events = page;
    });
  }

  void _setEventPageSize(int size) {
    setState(() => _eventPageSize = size);
    _loadEvents(offset: 0);
  }

  Future<void> _confirmAndApply(
    BuildContext context,
    MemoryEventEdit edit,
    String title,
    String message,
  ) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(l10n.confirm),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    if (!mounted) return;
    await widget.notifier.applyMemoryEventEdit(edit);
    // On success the notifier refreshes inspection → reloadKey changes →
    // didUpdateWidget fires and reloads this detail automatically.
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};
    final scheme = widget.theme.colorScheme;
    final character = _selectedCharacter;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.history_outlined, color: scheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.memoryEvents,
                    style: widget.theme.textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            // Two-pane row: characters left, events right
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Left pane: character search + pagination + list
                  SizedBox(
                    width: 280,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        TextField(
                          controller: _characterSearch,
                          decoration: InputDecoration(
                            labelText: l10n.searchCharacters,
                            isDense: true,
                            prefixIcon: const Icon(Icons.search, size: 18),
                            suffixIcon: _searchingCharacters
                                ? const Padding(
                                    padding: EdgeInsets.all(10),
                                    child: SizedBox(
                                      width: 16,
                                      height: 16,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                      ),
                                    ),
                                  )
                                : IconButton(
                                    icon: const Icon(
                                      Icons.arrow_forward,
                                      size: 18,
                                    ),
                                    onPressed: () => _loadCharacters(
                                      offset: 0,
                                      newQuery: true,
                                    ),
                                  ),
                          ),
                          onSubmitted: (_) =>
                              _loadCharacters(offset: 0, newQuery: true),
                        ),
                        if (_characters.error != null) ...[
                          const SizedBox(height: 4),
                          Text(
                            _characters.error!,
                            style: TextStyle(color: scheme.error, fontSize: 12),
                          ),
                        ],
                        const SizedBox(height: 4),
                        _PaginationBar(
                          offset: _characters.offset,
                          count: _characters.characters.length,
                          total: _characters.total,
                          pageSize: _charPageSize,
                          busy: _loadingCharacters,
                          onPage: (o) => _loadCharacters(offset: o),
                          onPageSize: (s) {
                            setState(() => _charPageSize = s);
                            _loadCharacters(offset: 0);
                          },
                        ),
                        const SizedBox(height: 4),
                        Expanded(
                          child:
                              _loadingCharacters &&
                                  _characters.characters.isEmpty
                              ? const Center(child: CircularProgressIndicator())
                              : ListView.separated(
                                  itemCount: _characters.characters.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (context, index) {
                                    final c = _characters.characters[index];
                                    final isSelected = c.id == character;
                                    return ListTile(
                                      dense: true,
                                      selected: isSelected,
                                      title: Text(
                                        l10n.characterWithCount(
                                          _localizedProgressionName(
                                            locCatalog,
                                            lang,
                                            c.id,
                                            c.id,
                                          ),
                                          c.eventCount,
                                        ),
                                      ),
                                      onTap: () => _selectCharacter(c.id),
                                    );
                                  },
                                ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 12),
                  const VerticalDivider(width: 1),
                  const SizedBox(width: 12),
                  // Right pane: header + pagination + events list
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          character != null
                              ? l10n.eventsForCharacter(
                                  _localizedProgressionName(
                                    locCatalog,
                                    lang,
                                    character,
                                    character,
                                  ),
                                )
                              : l10n.selectCharacterToSeeEvents,
                          style: widget.theme.textTheme.labelLarge,
                        ),
                        if (character != null) ...[
                          if (_events.error != null)
                            Padding(
                              padding: const EdgeInsets.only(top: 4),
                              child: Text(
                                _events.error!,
                                style: TextStyle(color: scheme.error),
                              ),
                            ),
                          const SizedBox(height: 4),
                          _PaginationBar(
                            offset: _events.offset,
                            count: _events.events.length,
                            total: _events.total,
                            pageSize: _eventPageSize,
                            busy: _loadingEvents,
                            onPage: (o) => _loadEvents(offset: o),
                            onPageSize: _setEventPageSize,
                          ),
                          const SizedBox(height: 4),
                          // Events list — only scrollable in this pane
                          Expanded(
                            child: _loadingEvents && _events.events.isEmpty
                                ? const Center(
                                    child: CircularProgressIndicator(),
                                  )
                                : ListView.separated(
                                    itemCount: _events.events.length,
                                    separatorBuilder: (_, _) =>
                                        const Divider(height: 1),
                                    itemBuilder: (context, index) {
                                      final event = _events.events[index];
                                      final tagLabel = event.tags.isEmpty
                                          ? l10n.noTags
                                          : event.tags.join(', ');
                                      final timeStr = event.timeSeconds != null
                                          ? event.timeSeconds!.toStringAsFixed(
                                              0,
                                            )
                                          : '?';
                                      final affected = event.affected ?? '';
                                      return ListTile(
                                        dense: true,
                                        title: SelectableText(
                                          tagLabel,
                                          maxLines: 1,
                                        ),
                                        subtitle: SelectableText(
                                          l10n.eventSubtitle(timeStr, affected),
                                          maxLines: 1,
                                        ),
                                        trailing: widget.editable
                                            ? Row(
                                                mainAxisSize: MainAxisSize.min,
                                                children: [
                                                  IconButton(
                                                    icon: const Icon(
                                                      Icons.delete_outline,
                                                      size: 20,
                                                    ),
                                                    tooltip: l10n.removeEvent,
                                                    onPressed: _loadingEvents
                                                        ? null
                                                        : () => _confirmAndApply(
                                                            context,
                                                            MemoryEventEdit.remove(
                                                              arrayPath: _events
                                                                  .arrayPath,
                                                              index:
                                                                  event.index,
                                                            ),
                                                            l10n.removeMemoryEventTitle,
                                                            l10n.removeMemoryEventBody,
                                                          ),
                                                  ),
                                                  IconButton(
                                                    icon: const Icon(
                                                      Icons.copy_outlined,
                                                      size: 20,
                                                    ),
                                                    tooltip: l10n.duplicateEvent,
                                                    onPressed: _loadingEvents
                                                        ? null
                                                        : () => _confirmAndApply(
                                                            context,
                                                            MemoryEventEdit.duplicate(
                                                              arrayPath: _events
                                                                  .arrayPath,
                                                              index:
                                                                  event.index,
                                                            ),
                                                            l10n.duplicateMemoryEventTitle,
                                                            l10n.duplicateMemoryEventBody,
                                                          ),
                                                  ),
                                                ],
                                              )
                                            : null,
                                      );
                                    },
                                  ),
                          ),
                        ],
                        if (character == null)
                          Expanded(
                            child: Center(
                              child: Text(
                                l10n.selectCharacterFromList,
                                style: TextStyle(
                                  color: scheme.onSurfaceVariant,
                                ),
                              ),
                            ),
                          ),
                      ],
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

// ---------------------------------------------------------------------------
// Factions detail — list of guilds the player has crimes against + forgive
// ---------------------------------------------------------------------------

/// Localized friendly name for a camp-level guild tag (or the `Other` bucket),
/// falling back to the core-supplied [label], then the raw [guild] tag.
String _localizedGuildLabel(
  AppLocalizations l10n,
  String guild,
  String label,
) {
  switch (guild) {
    case 'Guild.Human.OldCamp':
      return l10n.factionGuildOldCamp;
    case 'Guild.Human.NewCamp':
      return l10n.factionGuildNewCamp;
    case 'Guild.Human.SwampCamp':
      return l10n.factionGuildSwampCamp;
    case 'Other':
      return l10n.factionGuildOther;
    default:
      return label.isNotEmpty ? label : guild;
  }
}

/// A compact "·"-joined breakdown of UN-FORGIVEN crimes by type, omitting zero
/// categories, e.g. "3 Morde · 1 Übergriff · 5 Diebstähle". Empty when the
/// guild has no un-forgiven crimes.
String _crimeBreakdownText(AppLocalizations l10n, FactionGuild g) {
  final c = g.crimes;
  final parts = <String>[];
  if (c.murder > 0) parts.add(l10n.crimeMurder(c.murder));
  if (c.assault > 0) parts.add(l10n.crimeAssault(c.assault));
  if (c.theft > 0) parts.add(l10n.crimeTheft(c.theft));
  if (c.trespassing > 0) parts.add(l10n.crimeTrespassing(c.trespassing));
  if (c.threat > 0) parts.add(l10n.crimeThreat(c.threat));
  if (c.other > 0) parts.add(l10n.crimeOther(c.other));
  return parts.join(' · ');
}

/// A small rounded status pill (Feindselig / Friedlich / being-forgiven).
class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.5)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontWeight: FontWeight.w600,
          fontSize: 12,
        ),
      ),
    );
  }
}

class _FactionsDetail extends ConsumerStatefulWidget {
  const _FactionsDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<_FactionsDetail> createState() => _FactionsDetailState();
}

class _FactionsDetailState extends ConsumerState<_FactionsDetail> {
  FactionsPage _page = const FactionsPage();
  bool _loading = false;
  int _reloadEpoch = 0;

  /// Guild tags with a queued (pending) forgive, DERIVED from the global
  /// pending-edit registry rather than cached locally. A partial save refreshes
  /// the inspection and re-applies still-uncommitted pending edits (including
  /// `factions.forgive:*`); deriving from the registry keeps the optimistic
  /// "being forgiven…" reflect in sync across that refresh, whereas a local
  /// cache cleared on reload silently lost it.
  Set<String> get _pendingForgiven => widget.notifier.pendingForgiveGuilds();

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant _FactionsDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      // Reload the fresh tallies. The pending-forgive reflect is derived from
      // the registry (see _pendingForgiven), so it stays correct across the
      // post-save refresh without any local state to clear.
      _load();
    }
  }

  Future<void> _load() async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final page = await widget.notifier.loadFactions();
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _page = page;
    });
  }

  void _forgive(FactionGuild guild) {
    // Registers the pending edit in the global registry; _pendingForgiven is
    // derived from it, so just rebuild to reflect the queued forgive.
    widget.notifier.setPendingFactionForgive(guild.guild);
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final scheme = widget.theme.colorScheme;
    // Only guilds with open crimes are actionable; keep all in the list so the
    // player sees forgiven/total context too.
    final guilds = _page.guilds;
    // Derive the queued-forgive set once from the registry for this build.
    final pendingForgiven = _pendingForgiven;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.gavel_outlined, color: scheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.factionsSidebar,
                    style: widget.theme.textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            if (_page.error != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  _page.error!,
                  style: TextStyle(color: scheme.error),
                ),
              ),
            Expanded(
              child: _loading && guilds.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : guilds.isEmpty
                  ? Center(
                      child: Text(
                        l10n.factionsEmpty,
                        style: TextStyle(color: scheme.onSurfaceVariant),
                      ),
                    )
                  : ListView.separated(
                      itemCount: guilds.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final g = guilds[index];
                        final isPending = pendingForgiven.contains(g.guild);
                        // Optimistic: a queued forgive clears the record →
                        // friendly, no un-forgiven crimes, button disabled.
                        final isHostile = isPending ? false : g.isHostile;
                        // Forgiving is allowed whenever there is any un-forgiven
                        // crime — even a "friendly" guild's record can be wiped.
                        final canForgive =
                            widget.editable && !isPending && g.unforgiven > 0;
                        final breakdown = _crimeBreakdownText(l10n, g);
                        return ListTile(
                          leading: Icon(
                            isHostile
                                ? Icons.gpp_bad_outlined
                                : Icons.verified_user_outlined,
                            color: isHostile ? scheme.error : Colors.green,
                          ),
                          title: Text(
                            _localizedGuildLabel(l10n, g.guild, g.label),
                          ),
                          subtitle: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const SizedBox(height: 4),
                              // Prominent hostile/friendly status badge.
                              _StatusBadge(
                                label: isPending
                                    ? l10n.factionsForgiveQueued
                                    : (isHostile
                                          ? l10n.factionHostile
                                          : l10n.factionFriendly),
                                color: isPending
                                    ? scheme.primary
                                    : (isHostile ? scheme.error : Colors.green),
                              ),
                              // Compact un-forgiven crime-type breakdown.
                              if (!isPending && breakdown.isNotEmpty)
                                Padding(
                                  padding: const EdgeInsets.only(top: 4),
                                  child: Text(
                                    breakdown,
                                    style: widget.theme.textTheme.bodySmall
                                        ?.copyWith(
                                          color: scheme.onSurfaceVariant,
                                        ),
                                  ),
                                ),
                            ],
                          ),
                          isThreeLine: true,
                          trailing: widget.editable
                              ? FilledButton.tonal(
                                  onPressed:
                                      canForgive ? () => _forgive(g) : null,
                                  child: Text(l10n.factionsForgiveButton),
                                )
                              : null,
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Quest filter row: status chips
// ---------------------------------------------------------------------------

class _QuestFilterRow extends StatelessWidget {
  const _QuestFilterRow({
    required this.page,
    required this.stateFilter,
    required this.busy,
    required this.onStateChanged,
  });

  final ProgressionQuestPage page;
  final String? stateFilter;
  final bool busy;
  final void Function(String label) onStateChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    // Status FilterChips — show labels with count > 0 OR currently selected
    // (so a selected chip whose count dropped to 0 stays visible for deselect).
    final chips = [
      for (final label in _filterStateLabels)
        if ((page.stateCounts[label] ?? 0) > 0 || stateFilter == label)
          FilterChip(
            label: Text(
              l10n.stateLabelWithCount(
                _localizedShortLabel(l10n, label),
                stateFilter == label && (page.stateCounts[label] ?? 0) == 0
                    ? 0
                    : page.stateCounts[label] ?? 0,
              ),
            ),
            selected: stateFilter == label,
            onSelected: busy ? null : (_) => onStateChanged(label),
            visualDensity: VisualDensity.compact,
          ),
    ];

    return Wrap(
      spacing: 6,
      runSpacing: 4,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [...chips],
    );
  }
}

// ---------------------------------------------------------------------------
// _MessagePane (local helper, duplicated from editor_page.dart pattern)
// ---------------------------------------------------------------------------

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
