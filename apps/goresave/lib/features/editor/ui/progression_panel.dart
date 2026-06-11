import 'package:flutter/material.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/pending_edits.dart';
import '../domain/progression_models.dart';

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
            'Progression data needs decoded private payload data from the G1R codec host.',
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
          // Slim left sidebar
          SizedBox(
            width: 140,
            child: Card(
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 8),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
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
          const SizedBox(width: 12),
          // Detail area — fills remaining width and full height
          Expanded(
            child: switch (_selected) {
              _ProgSection.quests => _QuestsDetail(
                key: ValueKey(('quests', reloadKey)),
                notifier: widget.notifier,
                editable: widget.editable,
                reloadKey: reloadKey,
                theme: theme,
              ),
              _ProgSection.knowledge => _KnowledgeDetail(
                key: ValueKey(('knowledge', reloadKey)),
                notifier: widget.notifier,
                editable: widget.editable,
                reloadKey: reloadKey,
                theme: theme,
              ),
              _ProgSection.events => _EventsDetail(
                key: ValueKey(('events', reloadKey)),
                notifier: widget.notifier,
                editable: widget.editable,
                reloadKey: reloadKey,
                theme: theme,
              ),
            },
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
    return Material(
      color: selected ? scheme.secondaryContainer : Colors.transparent,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 12),
          child: Row(
            children: [
              Icon(
                icon,
                size: 18,
                color: selected
                    ? scheme.onSecondaryContainer
                    : scheme.onSurfaceVariant,
              ),
              const SizedBox(width: 8),
              Flexible(
                child: Text(
                  label,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
                    color: selected
                        ? scheme.onSecondaryContainer
                        : scheme.onSurface,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
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
  final Object reloadKey;
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
  bool _searching = false;
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
    setState(() {
      _loading = true;
      if (newQuery) _searching = true;
    });
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
      _searching = false;
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
                suffixIcon: _searching
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
  final Object reloadKey;
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
  int _reloadEpoch = 0;
  int _charPageSize = _defaultPageSize;
  int _entryPageSize = _defaultPageSize;
  String _activeCharQuery = '';

  @override
  void initState() {
    super.initState();
    _loadCharacters(offset: 0);
  }

  @override
  void didUpdateWidget(covariant _KnowledgeDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _selectedCharacter = null;
      _entries = const KnowledgeEntriesPage();
      _characterSearch.clear();
      _activeCharQuery = '';
      _loadCharacters(offset: 0);
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
    final epoch = ++_reloadEpoch;
    setState(() {
      _loadingCharacters = true;
      if (newQuery) _searchingCharacters = true;
    });
    final page = await widget.notifier.loadKnowledgeCharacters(
      query: _activeCharQuery,
      offset: offset,
      limit: _charPageSize,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingCharacters = false;
      _searchingCharacters = false;
      _characters = page;
    });
  }

  Future<void> _selectCharacter(String name) async {
    final epoch = ++_reloadEpoch;
    setState(() {
      _selectedCharacter = name;
      _loadingEntries = true;
      _entries = const KnowledgeEntriesPage();
    });
    final page = await widget.notifier.loadKnowledgeEntries(
      name,
      offset: 0,
      limit: _entryPageSize,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEntries = false;
      _entries = page;
    });
  }

  Future<void> _loadEntries({required int offset}) async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_reloadEpoch;
    setState(() => _loadingEntries = true);
    final page = await widget.notifier.loadKnowledgeEntries(
      character,
      offset: offset,
      limit: _entryPageSize,
    );
    if (!mounted || epoch != _reloadEpoch) return;
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

  String _pendingKey(String character, String entry) => '$character\t$entry';

  void _removeEntry(String entry) {
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

  void _addEntry(String entry) {
    final trimmed = entry.trim();
    if (trimmed.isEmpty) return;
    if (_entries.entries.contains(trimmed)) return;
    final key = _pendingKey(_selectedCharacter!, trimmed);
    if (_pending.containsKey(key)) return;
    setState(() {
      _pending[key] = KnowledgeEntryEdit.add(
        setPath: _entries.setPath,
        entry: trimmed,
      );
      _addController.clear();
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
    final removedEntries = <String>{};
    final addedEntries = <String>[];
    if (character != null) {
      final prefix = '$character\t';
      for (final e in _pending.entries) {
        if (!e.key.startsWith(prefix)) continue;
        if (!e.value.isAdd) removedEntries.add(e.value.entry);
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
                          if (widget.editable)
                            Row(
                              children: [
                                Expanded(
                                  child: TextField(
                                    controller: _addController,
                                    decoration: const InputDecoration(
                                      labelText: 'Add knowledge entry',
                                      isDense: true,
                                    ),
                                    onSubmitted: _addEntry,
                                  ),
                                ),
                                const SizedBox(width: 8),
                                IconButton(
                                  icon: const Icon(Icons.add),
                                  tooltip: 'Add',
                                  onPressed: () =>
                                      _addEntry(_addController.text),
                                ),
                              ],
                            ),
                          if (_entries.error != null)
                            Padding(
                              padding: const EdgeInsets.only(top: 4),
                              child: Text(
                                _entries.error!,
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
                          // Entries list — only scrollable in this pane
                          Expanded(
                            child: _loadingEntries && _entries.entries.isEmpty
                                ? const Center(
                                    child: CircularProgressIndicator(),
                                  )
                                : ListView.separated(
                                    itemCount:
                                        addedEntries.length +
                                        _entries.entries.length,
                                    separatorBuilder: (_, _) =>
                                        const Divider(height: 1),
                                    itemBuilder: (context, index) {
                                      // Pending-add tiles at the top
                                      if (index < addedEntries.length) {
                                        final entry = addedEntries[index];
                                        return ListTile(
                                          dense: true,
                                          tileColor: scheme.tertiaryContainer
                                              .withValues(alpha: 0.4),
                                          title: Text(
                                            entry,
                                            style: TextStyle(
                                              color: scheme.onTertiaryContainer,
                                            ),
                                          ),
                                          trailing: widget.editable
                                              ? IconButton(
                                                  icon: const Icon(
                                                    Icons.undo,
                                                    size: 18,
                                                  ),
                                                  tooltip: 'Undo add',
                                                  onPressed: () =>
                                                      _undoAdd(entry),
                                                )
                                              : null,
                                        );
                                      }
                                      final entry = _entries
                                          .entries[index - addedEntries.length];
                                      final isRemoved = removedEntries.contains(
                                        entry,
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
  final Object reloadKey;
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
  int _reloadEpoch = 0;
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
      _selectedCharacter = null;
      _events = const MemoryEventsPage();
      _characterSearch.clear();
      _activeCharQuery = '';
      _loadCharacters(offset: 0);
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
    final epoch = ++_reloadEpoch;
    setState(() {
      _loadingCharacters = true;
      if (newQuery) _searchingCharacters = true;
    });
    final page = await widget.notifier.loadMemoryCharacters(
      query: _activeCharQuery,
      offset: offset,
      limit: _charPageSize,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingCharacters = false;
      _searchingCharacters = false;
      _characters = page;
    });
  }

  Future<void> _selectCharacter(String id) async {
    final epoch = ++_reloadEpoch;
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
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEvents = false;
      _events = page;
    });
  }

  Future<void> _loadEvents({required int offset}) async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_reloadEpoch;
    setState(() => _loadingEvents = true);
    final page = await widget.notifier.loadMemoryEvents(
      character,
      offset: offset,
      limit: _eventPageSize,
    );
    if (!mounted || epoch != _reloadEpoch) return;
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
    required this.onStateChanged,
    required this.onGroupChanged,
  });

  final ProgressionQuestPage page;
  final String? stateFilter;
  final String? groupFilter;
  final void Function(String label) onStateChanged;
  final void Function(String? group) onGroupChanged;

  @override
  Widget build(BuildContext context) {
    // Status FilterChips — only show labels with count > 0.
    final chips = [
      for (final label in _filterStateLabels)
        if ((page.stateCounts[label] ?? 0) > 0)
          FilterChip(
            label: Text('$label ${page.stateCounts[label]}'),
            selected: stateFilter == label,
            onSelected: (_) => onStateChanged(label),
            visualDensity: VisualDensity.compact,
          ),
    ];

    // Group dropdown entries sorted by name.
    final sortedGroups = page.groupCounts.keys.toList()..sort();

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
          onChanged: onGroupChanged,
          items: [
            const DropdownMenuItem<String?>(
              value: null,
              child: Text('All groups'),
            ),
            for (final g in sortedGroups)
              DropdownMenuItem<String?>(
                value: g,
                child: Text('$g (${page.groupCounts[g]})'),
              ),
          ],
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
