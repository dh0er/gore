import 'package:flutter/material.dart';

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

/// Sidebar section entries for the Progression tab.
enum _ProgSection { quests, knowledge, events }

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
    if (!widget.inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Progression data needs decoded private payload data from the codec.',
      );
    }
    if (!widget.inspection.privateProgression.available) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Structured progression data needs a fully decoded save with a '
            'verified typed parse.',
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
                      label: 'Quests',
                      selected: _selected == _ProgSection.quests,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.quests),
                    ),
                    _SidebarTile(
                      icon: Icons.school_outlined,
                      label: 'Knowledge',
                      selected: _selected == _ProgSection.knowledge,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.knowledge),
                    ),
                    _SidebarTile(
                      icon: Icons.history_outlined,
                      label: 'Events',
                      selected: _selected == _ProgSection.events,
                      onTap: () =>
                          setState(() => _selected = _ProgSection.events),
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
          tooltip: 'First page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.first_page),
          onPressed: busy || !_hasPrevious ? null : () => onPage(0),
        ),
        IconButton(
          tooltip: 'Previous page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_left),
          onPressed: busy || !_hasPrevious
              ? null
              : () => onPage((_pageIndex - 1) * pageSize),
        ),
        IconButton(
          tooltip: 'Next page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_right),
          onPressed: busy || !_hasNext
              ? null
              : () => onPage((_pageIndex + 1) * pageSize),
        ),
        IconButton(
          tooltip: 'Last page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.last_page),
          onPressed: busy || !_hasNext
              ? null
              : () => onPage((_pageCount - 1) * pageSize),
        ),
        const SizedBox(width: 4),
        Text('Page ${_pageIndex + 1} / $_pageCount', style: muted),
        const SizedBox(width: 8),
        Text('$first–$last of $total', style: muted),
        const SizedBox(width: 8),
        Text('Per page:', style: muted),
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

class _QuestsDetail extends StatefulWidget {
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
  State<_QuestsDetail> createState() => _QuestsDetailState();
}

class _QuestsDetailState extends State<_QuestsDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _search = TextEditingController();
  ProgressionQuestPage _page = const ProgressionQuestPage();
  final Map<String, QuestStateChange> _pending = {};
  bool _loading = false;
  int _reloadEpoch = 0;
  int _pageSize = _defaultPageSize;
  String _activeQuery = '';
  String? _stateFilter;
  String? _groupFilter;

  @override
  void initState() {
    super.initState();
    _reload(offset: 0);
  }

  @override
  void didUpdateWidget(covariant _QuestsDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _search.clear();
      _activeQuery = '';
      _stateFilter = null;
      _groupFilter = null;
      _reload(offset: 0);
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _reload({required int offset, bool newQuery = false}) async {
    if (newQuery) _activeQuery = _search.text.trim();
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final page = await widget.notifier.loadProgressionQuests(
      query: _activeQuery,
      offset: offset,
      limit: _pageSize,
      state: _stateFilter,
      group: _groupFilter,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _page = page;
    });
  }

  void _goToPage(int newOffset) => _reload(offset: newOffset);

  void _setPageSize(int size) {
    setState(() => _pageSize = size);
    _reload(offset: 0);
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

  @override
  Widget build(BuildContext context) {
    final scheme = widget.theme.colorScheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
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
                    'Quests',
                    style: widget.theme.textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && _pending.isNotEmpty)
                  Tooltip(
                    message: 'Reset quest changes',
                    child: IconButton(
                      icon: const Icon(Icons.undo_outlined),
                      onPressed: () {
                        setState(_pending.clear);
                        widget.notifier.clearPendingEdit('progression.quests');
                      },
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            // Search field with in-flight spinner
            TextField(
              controller: _search,
              decoration: InputDecoration(
                labelText: 'Search quests',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _loading
                    ? const Padding(
                        padding: EdgeInsets.all(12),
                        child: SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      )
                    : IconButton(
                        icon: const Icon(Icons.arrow_forward),
                        onPressed: () => _reload(offset: 0, newQuery: true),
                      ),
              ),
              onSubmitted: (_) => _reload(offset: 0, newQuery: true),
            ),
            if (_page.error != null) ...[
              const SizedBox(height: 8),
              Text(_page.error!, style: TextStyle(color: scheme.error)),
            ],
            const SizedBox(height: 8),
            // Filter row: status chips + group dropdown
            _QuestFilterRow(
              page: _page,
              stateFilter: _stateFilter,
              groupFilter: _groupFilter,
              busy: _loading,
              onStateChanged: (label) {
                setState(() {
                  _stateFilter = (_stateFilter == label) ? null : label;
                });
                _reload(offset: 0);
              },
              onGroupChanged: (group) {
                setState(() => _groupFilter = group);
                _reload(offset: 0);
              },
            ),
            const SizedBox(height: 4),
            _PaginationBar(
              offset: _page.offset,
              count: _page.quests.length,
              total: _page.total,
              pageSize: _pageSize,
              busy: _loading,
              onPage: _goToPage,
              onPageSize: _setPageSize,
            ),
            const SizedBox(height: 4),
            // Quest list — the only scrollable, fills remaining height
            Expanded(
              child: _loading && _page.quests.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount: _page.quests.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final quest = _page.quests[index];
                        final pendingState = _pending[quest.questClass]?.state;
                        final effectiveState =
                            pendingState ?? quest.currentState;
                        final inKnownStates =
                            effectiveState != null &&
                            questStates.contains(effectiveState);
                        return ListTile(
                          dense: true,
                          leading: const Icon(Icons.flag_outlined),
                          title: SelectableText(
                            quest.name.isEmpty ? quest.id : quest.name,
                            maxLines: 1,
                          ),
                          subtitle: SelectableText(quest.group, maxLines: 1),
                          trailing:
                              widget.editable && quest.writable && inKnownStates
                              ? DropdownButton<String>(
                                  value: effectiveState,
                                  underline: const SizedBox.shrink(),
                                  items: questStates
                                      .map(
                                        (s) => DropdownMenuItem(
                                          value: s,
                                          child: Text(shortStateLabel(s)),
                                        ),
                                      )
                                      .toList(),
                                  onChanged: (s) => _setQuestState(quest, s),
                                )
                              : Text(
                                  shortStateLabel(
                                    quest.currentState ?? 'unknown',
                                  ),
                                ),
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
// Knowledge detail — two-pane: NPC list left, entries right
// ---------------------------------------------------------------------------

class _KnowledgeDetail extends StatefulWidget {
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
  State<_KnowledgeDetail> createState() => _KnowledgeDetailState();
}

class _KnowledgeDetailState extends State<_KnowledgeDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _characterSearch = TextEditingController();
  KnowledgeCharactersPage _characters = const KnowledgeCharactersPage();
  String? _selectedCharacter;
  KnowledgeEntriesPage _entries = const KnowledgeEntriesPage();
  final Map<String, KnowledgeEntryEdit> _pending = {};
  final TextEditingController _addController = TextEditingController();
  bool _loadingCharacters = false;
  bool _searchingCharacters = false;
  bool _loadingEntries = false;
  // Per-loader epochs so a stale characters load never races an entries load.
  int _charsEpoch = 0;
  int _entriesEpoch = 0;
  int _charPageSize = _defaultPageSize;
  int _entryPageSize = _defaultPageSize;
  String _activeCharQuery = '';
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
    _loadCharacters(offset: 0);
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
    await _loadCharacters(offset: 0);
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
      _activeCharQuery = '';
      // A different save file means a different character set: drop the
      // selection. A same-file refresh (post-save reload) preserves it.
      if (widget.reloadKey.path != oldWidget.reloadKey.path) {
        _selectedCharacter = null;
      }
      _loadCharacters(offset: 0);
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
    final page = await widget.notifier.loadKnowledgeCharacters(
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
    // Issue C: defense-in-depth guard — setPath not yet loaded → reject.
    if (_entries.setPath.isEmpty) return;
    final trimmed = entry.trim();
    if (trimmed.isEmpty) return;
    // Fast path: already on the current page.
    // UE Names compare case-insensitively, so case-variants are duplicates.
    final trimmedLower = trimmed.toLowerCase();
    if (_entries.entries.any((e) => e.toLowerCase() == trimmedLower)) {
      setState(() => _addError = 'Already exists for this character.');
      return;
    }
    final character = _selectedCharacter!;
    // _pendingKey folds the entry to lower case, so this single lookup is
    // already case-insensitive.
    final key = _pendingKey(character, trimmed);
    if (_pending.containsKey(key)) {
      setState(() => _addError = 'Already in pending changes.');
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
        _addError = 'Duplicate check failed — try again: $checkError';
      });
      return;
    }
    if (exists) {
      setState(() => _addError = 'Already exists for this character.');
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
            // Header
            Row(
              children: [
                Icon(Icons.school_outlined, color: scheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Dialog Knowledge',
                    style: widget.theme.textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && _pending.isNotEmpty)
                  Tooltip(
                    message: 'Reset knowledge changes',
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
                              label: const Text('Add NPC'),
                              onPressed: _npcCatalog == null ? null : _addNpc,
                            ),
                          ),
                          const SizedBox(height: 8),
                        ],
                        TextField(
                          controller: _characterSearch,
                          decoration: InputDecoration(
                            labelText: 'Search NPCs',
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
                                    final isSelected = c.name == character;
                                    return ListTile(
                                      dense: true,
                                      selected: isSelected,
                                      title: Text(
                                        '${c.name} (${c.entryCount})',
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
                              ? 'Entries — $character'
                              : 'Select an NPC to see entries',
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
                                        decoration: const InputDecoration(
                                          labelText: 'Add knowledge entry',
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
                                            tooltip: 'Add',
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
                                      tooltip: 'Browse catalog',
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
                                'Pending adds (${addedEntries.length})',
                                style: widget.theme.textTheme.labelSmall
                                    ?.copyWith(color: scheme.onSurfaceVariant),
                              ),
                            ),
                            for (final entry in addedEntries)
                              ListTile(
                                dense: true,
                                tileColor: scheme.tertiaryContainer.withValues(
                                  alpha: 0.4,
                                ),
                                title: Text(
                                  entry,
                                  style: TextStyle(
                                    color: scheme.onTertiaryContainer,
                                  ),
                                ),
                                trailing: widget.editable
                                    ? IconButton(
                                        icon: const Icon(Icons.undo, size: 18),
                                        tooltip: 'Undo add',
                                        onPressed: () => _undoAdd(entry),
                                      )
                                    : null,
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
                                      return ListTile(
                                        dense: true,
                                        title: Text(
                                          entry,
                                          style: isRemoved
                                              ? const TextStyle(
                                                  decoration: TextDecoration
                                                      .lineThrough,
                                                )
                                              : null,
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
                                                    ? 'Undo remove'
                                                    : 'Remove entry',
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
                                'Select an NPC from the list',
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

class _EventsDetail extends StatefulWidget {
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
  State<_EventsDetail> createState() => _EventsDetailState();
}

class _EventsDetailState extends State<_EventsDetail> {
  static const _defaultPageSize = 50;

  final TextEditingController _characterSearch = TextEditingController();
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
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Confirm'),
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
                    'Memory Events',
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
                            labelText: 'Search characters',
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
                                      title: Text('${c.id} (${c.eventCount})'),
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
                              ? 'Events — $character'
                              : 'Select a character to see events',
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
                                          ? '(no tags)'
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
                                          't=${timeStr}s  $affected',
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
                                                    tooltip: 'Remove event',
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
                                                            'Remove memory event?',
                                                            'Remove this memory event? '
                                                                'A backup is written first.',
                                                          ),
                                                  ),
                                                  IconButton(
                                                    icon: const Icon(
                                                      Icons.copy_outlined,
                                                      size: 20,
                                                    ),
                                                    tooltip: 'Duplicate event',
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
                                                            'Duplicate memory event?',
                                                            'Duplicate this memory event? '
                                                                'A backup is written first.',
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
                                'Select a character from the list',
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
// Quest filter row: status chips + group dropdown
// ---------------------------------------------------------------------------

class _QuestFilterRow extends StatelessWidget {
  const _QuestFilterRow({
    required this.page,
    required this.stateFilter,
    required this.groupFilter,
    required this.busy,
    required this.onStateChanged,
    required this.onGroupChanged,
  });

  final ProgressionQuestPage page;
  final String? stateFilter;
  final String? groupFilter;
  final bool busy;
  final void Function(String label) onStateChanged;
  final void Function(String? group) onGroupChanged;

  @override
  Widget build(BuildContext context) {
    // Status FilterChips — show labels with count > 0 OR currently selected
    // (so a selected chip whose count dropped to 0 stays visible for deselect).
    final chips = [
      for (final label in _filterStateLabels)
        if ((page.stateCounts[label] ?? 0) > 0 || stateFilter == label)
          FilterChip(
            label: Text(
              stateFilter == label && (page.stateCounts[label] ?? 0) == 0
                  ? '$label 0'
                  : '$label ${page.stateCounts[label] ?? 0}',
            ),
            selected: stateFilter == label,
            onSelected: busy ? null : (_) => onStateChanged(label),
            visualDensity: VisualDensity.compact,
          ),
    ];

    // Group dropdown entries sorted by name.
    // Always include the currently selected group even if its count is now 0
    // (prevents DropdownButton crash on value not in items).
    final sortedGroups = page.groupCounts.keys.toList()..sort();
    final groupItems = <DropdownMenuItem<String?>>[
      const DropdownMenuItem<String?>(value: null, child: Text('All groups')),
      for (final g in sortedGroups)
        DropdownMenuItem<String?>(
          value: g,
          child: Text('$g (${page.groupCounts[g]})'),
        ),
      // Ensure selected group is present even when its count is 0.
      if (groupFilter != null && !page.groupCounts.containsKey(groupFilter))
        DropdownMenuItem<String?>(
          value: groupFilter,
          child: Text('$groupFilter (0)'),
        ),
    ];

    return Wrap(
      spacing: 6,
      runSpacing: 4,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        ...chips,
        DropdownButton<String?>(
          value: groupFilter,
          isDense: true,
          underline: const SizedBox.shrink(),
          hint: const Text('All groups'),
          onChanged: busy ? null : onGroupChanged,
          items: groupItems,
        ),
      ],
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
