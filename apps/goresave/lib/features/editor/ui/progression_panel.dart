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

String shortStateLabel(String state) {
  final idx = state.lastIndexOf('::');
  return idx < 0 ? state : state.substring(idx + 2);
}

/// Progression tab: structured quests / dialog knowledge / memory events.
/// Data loads lazily per card through the notifier's query_progression
/// wrappers. [reloadKey] is the [SaveInspection] instance itself; identity
/// comparison means every fresh inspection (even of the same file) clears
/// local pending state and reloads, matching the inventory card semantics.
class ProgressionPanel extends StatelessWidget {
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
  Widget build(BuildContext context) {
    final overview = inspection.privateProgression;
    if (!inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Progression data needs decoded private payload data from the G1R codec host.',
      );
    }
    if (!overview.available) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Structured progression data needs a fully decoded save with a '
            'verified typed parse.',
      );
    }
    final reloadKey = inspection;
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _OverviewCard(overview: overview),
        const SizedBox(height: 16),
        _QuestsCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
        const SizedBox(height: 16),
        _KnowledgeCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
        const SizedBox(height: 16),
        _EventsCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Overview card (stateless)
// ---------------------------------------------------------------------------

class _OverviewCard extends StatelessWidget {
  const _OverviewCard({required this.overview});

  final ProgressionOverview overview;

  @override
  Widget build(BuildContext context) {
    final states = overview.questStates;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.flag_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Progression summary',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _SummaryMetric(
                  label: 'Quests total',
                  value: overview.questTotal.toString(),
                ),
                for (final entry in states.entries)
                  _SummaryMetric(
                    label: entry.key,
                    value: entry.value.toString(),
                  ),
                _SummaryMetric(
                  label: 'Knowledge NPCs',
                  value: overview.knowledgeCharacters.toString(),
                ),
                _SummaryMetric(
                  label: 'Knowledge entries',
                  value: overview.knowledgeEntries.toString(),
                ),
                _SummaryMetric(
                  label: 'Memory characters',
                  value: overview.memoryCharacters.toString(),
                ),
                _SummaryMetric(
                  label: 'Memory events',
                  value: overview.memoryEvents.toString(),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Quests card (stateful, pending-edit pattern)
// ---------------------------------------------------------------------------

class _QuestsCard extends StatefulWidget {
  const _QuestsCard({
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  @override
  State<_QuestsCard> createState() => _QuestsCardState();
}

class _QuestsCardState extends State<_QuestsCard> {
  final TextEditingController _search = TextEditingController();
  ProgressionQuestPage _page = const ProgressionQuestPage();
  final List<ProgressionQuest> _quests = [];
  final Map<String, QuestStateChange> _pending = {};
  bool _loading = false;
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant _QuestsCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _reload();
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _reload({bool append = false}) async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final page = await widget.notifier.loadProgressionQuests(
      query: _search.text.trim(),
      offset: append ? _quests.length : 0,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _page = page;
      if (!append) _quests.clear();
      _quests.addAll(page.quests);
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

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.flag_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Quests',
                    style: Theme.of(context).textTheme.titleMedium,
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
            const SizedBox(height: 12),
            TextField(
              controller: _search,
              decoration: const InputDecoration(
                labelText: 'Search quests',
                prefixIcon: Icon(Icons.search),
              ),
              onSubmitted: (_) => _reload(),
            ),
            if (_page.error != null) ...[
              const SizedBox(height: 12),
              Text(
                _page.error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            const SizedBox(height: 8),
            SizedBox(
              height: 360,
              child: _loading && _quests.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount: _quests.length + (_page.hasMore ? 1 : 0),
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        if (index >= _quests.length) {
                          return TextButton(
                            onPressed: _loading
                                ? null
                                : () => _reload(append: true),
                            child: Text(
                              'Load more (${_quests.length} of ${_page.total})',
                            ),
                          );
                        }
                        final quest = _quests[index];
                        final pendingState = _pending[quest.questClass]?.state;
                        final effectiveState =
                            pendingState ?? quest.currentState;
                        // Guard: only show dropdown if the current value is a
                        // known questStates entry; otherwise fall back to text.
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
// Knowledge card (stateful, pending-edit pattern)
// ---------------------------------------------------------------------------

class _KnowledgeCard extends StatefulWidget {
  const _KnowledgeCard({
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  @override
  State<_KnowledgeCard> createState() => _KnowledgeCardState();
}

class _KnowledgeCardState extends State<_KnowledgeCard> {
  final TextEditingController _characterSearch = TextEditingController();
  KnowledgeCharactersPage _characters = const KnowledgeCharactersPage();
  String? _selectedCharacter;
  KnowledgeEntriesPage _entries = const KnowledgeEntriesPage();
  final Map<String, KnowledgeEntryEdit> _pending = {};
  final TextEditingController _addController = TextEditingController();
  bool _loadingCharacters = false;
  bool _loadingEntries = false;
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _loadCharacters();
  }

  @override
  void didUpdateWidget(covariant _KnowledgeCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _selectedCharacter = null;
      _entries = const KnowledgeEntriesPage();
      _characterSearch.clear();
      _loadCharacters();
    }
  }

  @override
  void dispose() {
    _characterSearch.dispose();
    _addController.dispose();
    super.dispose();
  }

  Future<void> _loadCharacters({bool append = false}) async {
    final epoch = ++_reloadEpoch;
    setState(() => _loadingCharacters = true);
    final page = await widget.notifier.loadKnowledgeCharacters(
      query: _characterSearch.text.trim(),
      offset: append ? _characters.characters.length : 0,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingCharacters = false;
      if (!append) {
        _characters = page;
      } else {
        _characters = KnowledgeCharactersPage(
          characters: [..._characters.characters, ...page.characters],
          total: page.total,
          offset: page.offset,
          limit: page.limit,
          error: page.error,
        );
      }
    });
  }

  Future<void> _selectCharacter(String name) async {
    final epoch = ++_reloadEpoch;
    setState(() {
      _selectedCharacter = name;
      _loadingEntries = true;
      _entries = const KnowledgeEntriesPage();
    });
    final page = await widget.notifier.loadKnowledgeEntries(name);
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEntries = false;
      _entries = page;
    });
  }

  Future<void> _loadMoreEntries() async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_reloadEpoch;
    setState(() => _loadingEntries = true);
    final page = await widget.notifier.loadKnowledgeEntries(
      character,
      offset: _entries.entries.length,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEntries = false;
      _entries = KnowledgeEntriesPage(
        character: page.character,
        entries: [..._entries.entries, ...page.entries],
        setPath: page.setPath,
        total: page.total,
        offset: page.offset,
        limit: page.limit,
        error: page.error,
      );
    });
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
        // Was a pending-add — undo it
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
    if (_pending.containsKey(key)) return; // already pending
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
    final scheme = Theme.of(context).colorScheme;
    final character = _selectedCharacter;

    // Compute sets for rendering — only include edits for the selected NPC.
    // Keys are '$character\t$entry' so we filter by prefix.
    final removedEntries = <String>{};
    final addedEntries = <String>[];
    if (character != null) {
      final prefix = '$character\t';
      for (final entry in _pending.entries) {
        if (!entry.key.startsWith(prefix)) continue;
        if (!entry.value.isAdd) removedEntries.add(entry.value.entry);
        if (entry.value.isAdd) addedEntries.add(entry.value.entry);
      }
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.chat_bubble_outline),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Dialog Knowledge',
                    style: Theme.of(context).textTheme.titleMedium,
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
            const SizedBox(height: 12),
            TextField(
              controller: _characterSearch,
              decoration: const InputDecoration(
                labelText: 'Search NPCs',
                prefixIcon: Icon(Icons.search),
              ),
              onSubmitted: (_) => _loadCharacters(),
            ),
            if (_characters.error != null) ...[
              const SizedBox(height: 8),
              Text(_characters.error!, style: TextStyle(color: scheme.error)),
            ],
            const SizedBox(height: 8),
            // Character list (height-capped)
            SizedBox(
              height: 200,
              child: _loadingCharacters && _characters.characters.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount:
                          _characters.characters.length +
                          (_characters.hasMore ? 1 : 0),
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        if (index >= _characters.characters.length) {
                          return TextButton(
                            onPressed: _loadingCharacters
                                ? null
                                : () => _loadCharacters(append: true),
                            child: Text(
                              'Load more (${_characters.characters.length}'
                              ' of ${_characters.total})',
                            ),
                          );
                        }
                        final c = _characters.characters[index];
                        final isSelected = c.name == character;
                        return ListTile(
                          dense: true,
                          selected: isSelected,
                          title: Text('${c.name} (${c.entryCount})'),
                          onTap: () => _selectCharacter(c.name),
                        );
                      },
                    ),
            ),
            if (character != null) ...[
              const SizedBox(height: 12),
              Text(
                'Entries for $character',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              if (_entries.error != null)
                Text(_entries.error!, style: TextStyle(color: scheme.error)),
              if (_loadingEntries && _entries.entries.isEmpty)
                const Center(child: CircularProgressIndicator())
              else
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    // Existing entries
                    for (final entry in _entries.entries)
                      if (removedEntries.contains(entry))
                        Chip(
                          label: Text(
                            entry,
                            style: const TextStyle(
                              decoration: TextDecoration.lineThrough,
                            ),
                          ),
                          deleteIcon: const Icon(Icons.undo, size: 16),
                          onDeleted: widget.editable
                              ? () => _removeEntry(entry)
                              : null,
                        )
                      else
                        Chip(
                          label: Text(entry),
                          onDeleted: widget.editable
                              ? () => _removeEntry(entry)
                              : null,
                        ),
                    // Pending adds
                    for (final entry in addedEntries)
                      Chip(
                        backgroundColor: scheme.tertiaryContainer,
                        label: Text(
                          entry,
                          style: TextStyle(color: scheme.onTertiaryContainer),
                        ),
                        deleteIcon: const Icon(Icons.undo, size: 16),
                        onDeleted: widget.editable
                            ? () => _undoAdd(entry)
                            : null,
                      ),
                  ],
                ),
              if (_entries.hasMore) ...[
                const SizedBox(height: 8),
                TextButton(
                  onPressed: _loadingEntries ? null : _loadMoreEntries,
                  child: Text(
                    'Load more (${_entries.entries.length}'
                    ' of ${_entries.total})',
                  ),
                ),
              ],
              if (widget.editable) ...[
                const SizedBox(height: 12),
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
                      onPressed: () => _addEntry(_addController.text),
                    ),
                  ],
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Events card (stateful, immediate-write pattern)
// ---------------------------------------------------------------------------

class _EventsCard extends StatefulWidget {
  const _EventsCard({
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  @override
  State<_EventsCard> createState() => _EventsCardState();
}

class _EventsCardState extends State<_EventsCard> {
  final TextEditingController _characterSearch = TextEditingController();
  MemoryCharactersPage _characters = const MemoryCharactersPage();
  String? _selectedCharacter;
  MemoryEventsPage _events = const MemoryEventsPage();
  bool _loadingCharacters = false;
  bool _loadingEvents = false;
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _loadCharacters();
  }

  @override
  void didUpdateWidget(covariant _EventsCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _selectedCharacter = null;
      _events = const MemoryEventsPage();
      _characterSearch.clear();
      _loadCharacters();
    }
  }

  @override
  void dispose() {
    _characterSearch.dispose();
    super.dispose();
  }

  Future<void> _loadCharacters({bool append = false}) async {
    final epoch = ++_reloadEpoch;
    setState(() => _loadingCharacters = true);
    final page = await widget.notifier.loadMemoryCharacters(
      query: _characterSearch.text.trim(),
      offset: append ? _characters.characters.length : 0,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingCharacters = false;
      if (!append) {
        _characters = page;
      } else {
        _characters = MemoryCharactersPage(
          characters: [..._characters.characters, ...page.characters],
          total: page.total,
          offset: page.offset,
          limit: page.limit,
          error: page.error,
        );
      }
    });
  }

  Future<void> _selectCharacter(String id) async {
    final epoch = ++_reloadEpoch;
    setState(() {
      _selectedCharacter = id;
      _loadingEvents = true;
      _events = const MemoryEventsPage(); // clear stale page immediately
    });
    final page = await widget.notifier.loadMemoryEvents(id);
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEvents = false;
      _events = page;
    });
  }

  Future<void> _loadMoreEvents() async {
    final character = _selectedCharacter;
    if (character == null) return;
    final epoch = ++_reloadEpoch;
    setState(() => _loadingEvents = true);
    final page = await widget.notifier.loadMemoryEvents(
      character,
      offset: _events.events.length,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loadingEvents = false;
      _events = MemoryEventsPage(
        character: page.character,
        events: [..._events.events, ...page.events],
        arrayPath: page.arrayPath,
        total: page.total,
        offset: page.offset,
        limit: page.limit,
        error: page.error,
      );
    });
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
    // didUpdateWidget fires and reloads this card automatically.
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final character = _selectedCharacter;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.memory_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Memory Events',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _characterSearch,
              decoration: const InputDecoration(
                labelText: 'Search characters',
                prefixIcon: Icon(Icons.search),
              ),
              onSubmitted: (_) => _loadCharacters(),
            ),
            if (_characters.error != null) ...[
              const SizedBox(height: 8),
              Text(_characters.error!, style: TextStyle(color: scheme.error)),
            ],
            const SizedBox(height: 8),
            // Character list (height-capped)
            SizedBox(
              height: 200,
              child: _loadingCharacters && _characters.characters.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount:
                          _characters.characters.length +
                          (_characters.hasMore ? 1 : 0),
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        if (index >= _characters.characters.length) {
                          return TextButton(
                            onPressed: _loadingCharacters
                                ? null
                                : () => _loadCharacters(append: true),
                            child: Text(
                              'Load more (${_characters.characters.length}'
                              ' of ${_characters.total})',
                            ),
                          );
                        }
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
            if (character != null) ...[
              const SizedBox(height: 12),
              Text(
                'Events for $character',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              if (_events.error != null)
                Text(_events.error!, style: TextStyle(color: scheme.error)),
              SizedBox(
                height: 360,
                child: _loadingEvents && _events.events.isEmpty
                    ? const Center(child: CircularProgressIndicator())
                    : ListView.separated(
                        itemCount:
                            _events.events.length + (_events.hasMore ? 1 : 0),
                        separatorBuilder: (_, _) => const Divider(height: 1),
                        itemBuilder: (context, index) {
                          if (index >= _events.events.length) {
                            return TextButton(
                              onPressed: _loadingEvents
                                  ? null
                                  : _loadMoreEvents,
                              child: Text(
                                'Load more (${_events.events.length} of ${_events.total})',
                              ),
                            );
                          }
                          final event = _events.events[index];
                          final tagLabel = event.tags.isEmpty
                              ? '(no tags)'
                              : event.tags.join(', ');
                          final timeStr = event.timeSeconds != null
                              ? event.timeSeconds!.toStringAsFixed(0)
                              : '?';
                          final affected = event.affected ?? '';
                          return ListTile(
                            dense: true,
                            title: SelectableText(tagLabel, maxLines: 1),
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
                                                  arrayPath: _events.arrayPath,
                                                  index: event.index,
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
                                                  arrayPath: _events.arrayPath,
                                                  index: event.index,
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
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Local private helpers (duplicated from editor_page.dart)
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

class _SummaryMetric extends StatelessWidget {
  const _SummaryMetric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 120,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ],
      ),
    );
  }
}
