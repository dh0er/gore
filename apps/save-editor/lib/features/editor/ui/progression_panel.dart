/// Progression detail widgets, shared by two tabs: [QuestsDetail] and
/// [FactionsDetail] are mounted by the Welt (World) tab's sidebar sections,
/// while [KnowledgeDetail] and [EventsDetail] are detail-only panels keyed by
/// the shared character selection and mounted by the Charaktere (Characters)
/// tab's sub-tabs. The dissolved Progression tab shell used to live here; only
/// its detail widgets and their shared helpers (pagination bar, state/name
/// localization) remain.
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/loc/progression_loc.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/knowledge_catalog.dart';
import '../domain/pending_edits.dart';
import '../domain/progression_models.dart';
import 'add_knowledge_entry_dialog.dart';

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

class QuestsDetail extends ConsumerStatefulWidget {
  const QuestsDetail({
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
  ConsumerState<QuestsDetail> createState() => _QuestsDetailState();
}

class _QuestsDetailState extends ConsumerState<QuestsDetail> {
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
  void didUpdateWidget(covariant QuestsDetail oldWidget) {
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
            .where((e) => matchesQuery(e) && matchesState(e) && matchesGroup(e))
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
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
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
                  // No card title ("Quests" icon+text row): the Welt sidebar
                  // tile already names the section. The reset-pending-edits
                  // action that lived in that row now trails the search field.
                  Row(
                    children: [
                      Expanded(
                        // Search field — client-side, filters live as you type.
                        child: TextField(
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
                      ),
                      if (widget.editable && _pending.isNotEmpty) ...[
                        const SizedBox(width: 8),
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
                    ],
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
// Knowledge detail — entries for a single, externally-selected character
// ---------------------------------------------------------------------------

/// Dialog-knowledge detail for one character. The selected character is passed
/// in via [uniqueName] (its knowledge key, e.g. `NC_ORG_Lares_801` or `Hero`);
/// null means nothing is selected and the panel shows an empty state. There is
/// no character pane here — selection is owned by a shared master list — so
/// this is a detail-only panel that reacts to [uniqueName] changes.
///
/// [reloadKey] is the current [SaveInspection]; identity comparison means a
/// fresh inspection clears pending state and reloads the selected character's
/// entries.
class KnowledgeDetail extends ConsumerStatefulWidget {
  const KnowledgeDetail({
    super.key,
    required this.uniqueName,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  /// Knowledge key of the selected character, or null when nothing is selected.
  final String? uniqueName;
  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<KnowledgeDetail> createState() => _KnowledgeDetailState();
}

class _KnowledgeDetailState extends ConsumerState<KnowledgeDetail> {
  static const _defaultPageSize = 50;

  String? _selectedCharacter;
  KnowledgeEntriesPage _entries = const KnowledgeEntriesPage();
  final Map<String, KnowledgeEntryEdit> _pending = {};
  final TextEditingController _addController = TextEditingController();
  bool _loadingEntries = false;
  // Epoch guards the entries loader so a stale load never clobbers a newer one.
  int _entriesEpoch = 0;
  // Armed by _ensureCharacterEntry right before its applyAddKnowledgeCharacter
  // write refreshes the inspection. That refresh is SELF-INFLICTED: the
  // in-flight first-add flow reloads entries itself and queues its pending
  // edit against the fresh inspection AFTER didUpdateWidget fires for it —
  // so didUpdateWidget must treat exactly that one reloadKey change as "ours"
  // (no _pending.clear(), no re-select) instead of as staleness, or the
  // user's typed entry is silently dropped next to a freshly created EMPTY
  // knowledge set. Consumed by the first reloadKey change after arming.
  bool _expectSelfRefresh = false;
  int _entryPageSize = _defaultPageSize;
  // Used during the cross-page duplicate check in _addEntry.
  bool _checkingDuplicate = false;
  // Error text shown beneath the add field (duplicate-check failure, etc.).
  String? _addError;
  // True when the selected character has no CharacterKnowledgeByUniqueName
  // entry yet (the common case for an NPC the hero never interacted with). The
  // core reports this via a benign "has no knowledge entry" error; we treat it
  // as "no knowledge yet" and still offer the add affordance, creating the
  // character entry on the first add. Distinct from a real load failure.
  bool _noKnowledgeYet = false;
  // Bundled catalog for the knowledge-entry picker dialog. Null until loaded
  // asynchronously in initState.
  KnowledgeCatalog? _knowledgeCatalog;

  /// Substring the core uses in its error when a character exists in the save
  /// but has no CharacterKnowledgeByUniqueName entry yet (see gore-save
  /// `query_progression` knowledge branch). Used to tell that benign
  /// "no knowledge yet" state apart from a genuine core/parse failure.
  static const _noEntryMarker = 'has no knowledge entry';

  @override
  void initState() {
    super.initState();
    _loadCatalog();
    _selectCharacter(widget.uniqueName);
  }

  Future<void> _loadCatalog() async {
    final knowledge = await KnowledgeCatalog.loadBundled();
    if (!mounted) return;
    setState(() {
      _knowledgeCatalog = knowledge;
    });
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
  void didUpdateWidget(covariant KnowledgeDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    final reloaded = widget.reloadKey != oldWidget.reloadKey;
    final selectionChanged = widget.uniqueName != oldWidget.uniqueName;
    // A reload caused by _ensureCharacterEntry's own write (the first-add
    // flow) is NOT staleness: that flow reloads entries itself and queues the
    // pending add afterwards. Clearing _pending or re-selecting (which bumps
    // _entriesEpoch and resets _entries) here would drop the in-flight add.
    final selfRefresh = reloaded && _expectSelfRefresh;
    if (reloaded) {
      _expectSelfRefresh = false;
      if (!selfRefresh) {
        // Pending edits belong to the old inspection — clear them.
        _pending.clear();
      }
    }
    // Reload the selected character's entries when either the shared selection
    // changed or a fresh EXTERNAL inspection arrived (post-save reload / new
    // file). The self-refresh reload happens inside _ensureCharacterEntry.
    if ((reloaded && !selfRefresh) || selectionChanged) {
      _selectCharacter(widget.uniqueName);
    }
  }

  @override
  void dispose() {
    _addController.dispose();
    super.dispose();
  }

  /// Loads the entries for [name] (the selected character's knowledge key).
  /// [name] == null clears to the empty state. Distinguishes the benign
  /// "no knowledge entry yet" core result (→ [_noKnowledgeYet], add affordance
  /// still shown) from a real load error (surfaced via [_entries.error]).
  Future<void> _selectCharacter(String? name) async {
    final epoch = ++_entriesEpoch;
    setState(() {
      _selectedCharacter = name;
      _loadingEntries = name != null;
      _entries = const KnowledgeEntriesPage();
      _addError = null;
      _noKnowledgeYet = false;
    });
    if (name == null) return;
    final page = await widget.notifier.loadKnowledgeEntries(
      name,
      offset: 0,
      limit: _entryPageSize,
    );
    if (!mounted || epoch != _entriesEpoch) return;
    // A "has no knowledge entry" error is the expected shape for a character
    // the hero never interacted with: fold it into the no-knowledge-yet state
    // (empty entries + add affordance) instead of surfacing a red error.
    final noEntry = page.error != null && page.error!.contains(_noEntryMarker);
    setState(() {
      _loadingEntries = false;
      _noKnowledgeYet = noEntry;
      _entries = noEntry ? const KnowledgeEntriesPage() : page;
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

  /// Creates the character's CharacterKnowledgeByUniqueName entry when it does
  /// not exist yet, then reloads entries so [_entries.setPath] is populated.
  /// This is the "no knowledge yet" first-add path that replaces the old
  /// "Add NPC" dialog: instead of adding an NPC to a list, the already-selected
  /// character gets its (empty) knowledge set created on demand.
  ///
  /// Returns true when [_entries.setPath] is ready to receive an add (either it
  /// was already populated, or the create+reload succeeded). Returns false and
  /// leaves [_addError] / notifier error set on any failure so the caller must
  /// not fall through to a pending add.
  Future<bool> _ensureCharacterEntry(String character) async {
    // Already has a knowledge set → nothing to create.
    if (_entries.setPath.isNotEmpty) return true;
    // Only the benign "no knowledge yet" state may auto-create. An empty
    // setPath from any other cause (still loading, real error) must not write.
    if (!_noKnowledgeYet) return false;
    // The write below refreshes the inspection (reloadKey changes). Arm the
    // self-refresh marker BEFORE the write so didUpdateWidget — which can fire
    // on any frame between here and the end of _addEntry — treats that one
    // refresh as ours instead of clearing _pending / re-selecting the same
    // character mid-flight.
    _expectSelfRefresh = true;
    final ok = await widget.notifier.applyAddKnowledgeCharacter(character);
    // A failed write never refreshed anything — disarm the marker so the NEXT
    // (genuinely external) reload is not mistaken for a self-refresh.
    if (!ok) _expectSelfRefresh = false;
    // The notifier sets state.error on failure; also guard unmount/reselect.
    if (!mounted || _selectedCharacter != character) return false;
    if (!ok) return false;
    // applyAddKnowledgeCharacter refreshed the inspection; reload here so the
    // populated setPath is available synchronously for the add below rather
    // than depending on the parent rebuild's timing. Bump the epoch so any
    // OLDER in-flight load can't clobber this fresh page — but do NOT compare
    // against it below: a same-character refresh-driven reload bumping the
    // epoch is not staleness for this flow. Only a real character switch or
    // unmount (both covered by the guard) may abort the add.
    ++_entriesEpoch;
    final page = await widget.notifier.loadKnowledgeEntries(
      character,
      offset: 0,
      limit: _entryPageSize,
    );
    if (!mounted || _selectedCharacter != character) {
      return false;
    }
    setState(() {
      _loadingEntries = false;
      _noKnowledgeYet = false;
      _entries = page;
    });
    // If the entry still has no setPath the create did not take — do not queue.
    return _entries.setPath.isNotEmpty;
  }

  Future<void> _addEntry(String entry) async {
    final l10n = AppLocalizations.of(context);
    final character = _selectedCharacter;
    if (character == null) return;
    final trimmed = entry.trim();
    if (trimmed.isEmpty) return;
    // No-knowledge-yet path: create the character's (empty) knowledge set first
    // so setPath becomes known, then fall through to the normal add. When the
    // set already exists this is a no-op.
    if (_entries.setPath.isEmpty) {
      final ready = await _ensureCharacterEntry(character);
      if (!ready) return;
    }
    // Issue C: defense-in-depth guard — setPath not loaded → reject.
    if (_entries.setPath.isEmpty) return;
    // Fast path: already on the current page.
    // UE Names compare case-insensitively, so case-variants are duplicates.
    final trimmedLower = trimmed.toLowerCase();
    if (_entries.entries.any((e) => e.toLowerCase() == trimmedLower)) {
      setState(() => _addError = l10n.alreadyExistsForCharacter);
      return;
    }
    // _pendingKey folds the entry to lower case, so this single lookup is
    // already case-insensitive.
    final key = _pendingKey(character, trimmed);
    if (_pending.containsKey(key)) {
      setState(() => _addError = l10n.alreadyInPendingChanges);
      return;
    }

    // Issue B: cross-page duplicate check via a server query. The stale
    // guards below check unmount and character switch ONLY — deliberately
    // not the entries epoch: the first-add flow's own inspection refresh may
    // reload the same character's entries mid-check, and that must not drop
    // the add (see _expectSelfRefresh).
    final checkCharacter = character;
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
        if (!mounted || _selectedCharacter != checkCharacter) {
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
    if (!mounted || _selectedCharacter != checkCharacter) {
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

    // Re-read the setPath AFTER all awaits above: on the first-add path
    // _entries was replaced with the post-refresh page, so the queued edit
    // addresses the FRESH inspection. If an interleaved reload left it empty
    // (transient reset), do not queue a pathless edit — mirrors the Issue C
    // guard above.
    final setPath = _entries.setPath;
    if (setPath.isEmpty) return;
    setState(() {
      _pending[key] = KnowledgeEntryEdit.add(setPath: setPath, entry: trimmed);
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
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
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

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // No card title: the sub-tab label already says "Dialogwissen".
            // The reset-pending-edits action moved into the add-entry row.
            // Entries for the externally-selected character. The character list
            // lives in a shared master pane elsewhere; this panel is detail-only
            // and keyed off widget.uniqueName.
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // No per-character info label: the ActorDetailHeader above
                  // the card already identifies the selection. Only the
                  // null-selection hint remains.
                  if (character == null)
                    Text(
                      l10n.selectNpcToSeeEntries,
                      style: widget.theme.textTheme.labelLarge,
                    ),
                  if (character != null) ...[
                    if (widget.editable) ...[
                      // Issue C: disabled while entries are loading or a
                      // duplicate check is in flight. An empty setPath
                      // normally disables the field, EXCEPT in the
                      // no-knowledge-yet state: there the first add is
                      // allowed and creates the character's knowledge set
                      // on demand (see _ensureCharacterEntry).
                      Builder(
                        builder: (context) {
                          final addDisabled =
                              _loadingEntries ||
                              (_entries.setPath.isEmpty && !_noKnowledgeYet) ||
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
                                          : () =>
                                                _addEntry(_addController.text),
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
                              // Reset-pending-edits: lived in the removed
                              // card-title row; now rides at the end of the
                              // add-entry row (same icon + tooltip).
                              if (_pending.isNotEmpty)
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
                          style: widget.theme.textTheme.labelSmall?.copyWith(
                            color: scheme.onSurfaceVariant,
                          ),
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
                              tileColor: scheme.tertiaryContainer.withValues(
                                alpha: 0.4,
                              ),
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
                                      icon: const Icon(Icons.undo, size: 18),
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
                          ? const Center(child: CircularProgressIndicator())
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
                                            decoration:
                                                TextDecoration.lineThrough,
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
                                            color: scheme.onSurfaceVariant,
                                            fontSize: 11,
                                            decoration: isRemoved
                                                ? TextDecoration.lineThrough
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
                                          onPressed: () => _removeEntry(entry),
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
                          style: TextStyle(color: scheme.onSurfaceVariant),
                        ),
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
// Events detail — events for a single, externally-selected character
// ---------------------------------------------------------------------------

/// Memory-event detail for one character. The selected character is passed in
/// via [globalId] (its GlobalId); null means nothing is selected (or a player
/// with no resolved id) and the panel shows an empty state. Detail-only: the
/// character list is owned by a shared master pane elsewhere.
class EventsDetail extends ConsumerStatefulWidget {
  const EventsDetail({
    super.key,
    required this.globalId,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
  });

  /// GlobalId of the selected character, or null when nothing is selected.
  final String? globalId;

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;

  @override
  ConsumerState<EventsDetail> createState() => _EventsDetailState();
}

class _EventsDetailState extends ConsumerState<EventsDetail> {
  static const _defaultPageSize = 50;

  String? _selectedCharacter;
  MemoryEventsPage _events = const MemoryEventsPage();
  bool _loadingEvents = false;
  // Epoch guards the events loader so a stale load never clobbers a newer one.
  int _eventsEpoch = 0;
  int _eventPageSize = _defaultPageSize;
  // True when the selected character has no LongTermMemoryByGlobalId entry yet
  // (the common case for an NPC the hero never interacted with). The core
  // reports this via a benign "has no memory entry" error; we treat it as
  // "no events yet" and render a neutral empty state instead of a red error.
  // Distinct from a real load failure. Mirrors KnowledgeDetail's
  // [_noKnowledgeYet] handling of the equivalent knowledge error.
  bool _noEventsYet = false;

  /// Substring the core uses in its error when a character exists in the save
  /// but has no LongTermMemoryByGlobalId entry yet (see gore-save
  /// `query_progression` events branch). Used to tell that benign "no events
  /// yet" state apart from a genuine core/parse failure.
  static const _noEntryMarker = 'has no memory entry';

  @override
  void initState() {
    super.initState();
    _selectCharacter(widget.globalId);
  }

  @override
  void didUpdateWidget(covariant EventsDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    final reloaded = widget.reloadKey != oldWidget.reloadKey;
    final selectionChanged = widget.globalId != oldWidget.globalId;
    // Reload the selected character's events when either the shared selection
    // changed or a fresh inspection arrived (post-edit reload / new file). If
    // the character was since deleted the existing error rendering handles it.
    if (reloaded || selectionChanged) {
      _selectCharacter(widget.globalId);
    }
  }

  Future<void> _selectCharacter(String? id) async {
    final epoch = ++_eventsEpoch;
    setState(() {
      _selectedCharacter = id;
      _loadingEvents = id != null;
      _events = const MemoryEventsPage(); // clear stale page immediately
      _noEventsYet = false;
    });
    if (id == null) return;
    final page = await widget.notifier.loadMemoryEvents(
      id,
      offset: 0,
      limit: _eventPageSize,
    );
    if (!mounted || epoch != _eventsEpoch) return;
    // A "has no memory entry" error is the expected shape for a character the
    // hero never interacted with: fold it into the no-events-yet state (a
    // neutral empty list) instead of surfacing a red error.
    final noEntry = page.error != null && page.error!.contains(_noEntryMarker);
    setState(() {
      _loadingEvents = false;
      _noEventsYet = noEntry;
      _events = noEntry ? const MemoryEventsPage() : page;
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
    final scheme = widget.theme.colorScheme;
    final character = _selectedCharacter;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // No card title: the sub-tab label already says "Ereignisse", and
            // the ActorDetailHeader above the card identifies the selection.
            // Events for the externally-selected character. The character list
            // lives in a shared master pane elsewhere; this panel is detail-only
            // and keyed off widget.globalId.
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Only the null-selection hint remains (see card-title note).
                  if (character == null)
                    Text(
                      l10n.selectCharacterToSeeEvents,
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
                    // Events list — only scrollable in this pane. In the
                    // benign no-memory-entry state the list is replaced by a
                    // neutral "no events" hint (there is no add flow for
                    // events, so unlike KnowledgeDetail no affordance remains).
                    Expanded(
                      child: _noEventsYet
                          ? Center(
                              child: Text(
                                l10n.characterNoEventsBody,
                                style: TextStyle(
                                  color: scheme.onSurfaceVariant,
                                ),
                              ),
                            )
                          : _loadingEvents && _events.events.isEmpty
                          ? const Center(child: CircularProgressIndicator())
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
                                    ? event.timeSeconds!.toStringAsFixed(0)
                                    : '?';
                                final affected = event.affected ?? '';
                                return ListTile(
                                  dense: true,
                                  title: SelectableText(tagLabel, maxLines: 1),
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
                                                        arrayPath:
                                                            _events.arrayPath,
                                                        index: event.index,
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
                                                        arrayPath:
                                                            _events.arrayPath,
                                                        index: event.index,
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
                          style: TextStyle(color: scheme.onSurfaceVariant),
                        ),
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
String _localizedGuildLabel(AppLocalizations l10n, String guild, String label) {
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

class FactionsDetail extends ConsumerStatefulWidget {
  const FactionsDetail({
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
  ConsumerState<FactionsDetail> createState() => _FactionsDetailState();
}

class _FactionsDetailState extends ConsumerState<FactionsDetail> {
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
  void didUpdateWidget(covariant FactionsDetail oldWidget) {
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
            // No card title ("Fraktionen" icon+text row): the Welt sidebar
            // tile already names the section.
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
                                  onPressed: canForgive
                                      ? () => _forgive(g)
                                      : null,
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
