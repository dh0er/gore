import 'dart:convert';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../../app/domain/ui_settings.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../loc/game_lang.dart';
import '../../loc/primary_set.dart';
import '../../loc/ui/lang_fields.dart';
import '../../project/dialog_topics_notifier.dart';
import '../../voice/ui/voice_line_editor.dart';
import '../domain/dialog_catalog_provider.dart';

/// Currently selected dialog line (loc id), shared by all [DialogeTab]
/// instances (the main tab and filtered embeddings such as the Changes tab)
/// so the selection survives tab switches. The main tab owns clearing it;
/// filtered views must never write null here just because the id fell out of
/// their filter — they guard at view level instead (see [_DialogEditor]).
final selectedDialogIdProvider = StateProvider<String?>((ref) => null);

/// Browse & edit the game's dialog / bark lines across languages. Edits are
/// staged into the shared [locEditsProvider].
class DialogeTab extends ConsumerWidget {
  const DialogeTab({super.key, this.onlyIds});

  /// When non-null, the browser shows only dialog lines whose (lowercased)
  /// loc id is in this set — the same key space as [locEditsProvider] edit
  /// keys. The restriction applies before grouping, so the speaker sidebar
  /// only lists groups with at least one filtered line and counts reflect
  /// the filtered lines. Null (default) shows the full catalog.
  final Set<String>? onlyIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final catalogAsync = ref.watch(locCatalogProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _DialogTopicsSection(
          storageScope: onlyIds == null ? 'main' : 'filtered',
        ),
        const Divider(height: 1),
        Expanded(
          child: catalogAsync.when(
            loading: () => const Center(child: CircularProgressIndicator()),
            error: (e, _) =>
                Center(child: Text('Failed to load localization: $e')),
            data: (_) {
              return Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SizedBox(width: 560, child: _DialogBrowser(onlyIds: onlyIds)),
                  const VerticalDivider(width: 1),
                  Expanded(child: _DialogEditor(onlyIds: onlyIds)),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

class _DialogTopicsSection extends ConsumerWidget {
  const _DialogTopicsSection({required this.storageScope});

  final String storageScope;

  Future<void> _openEditor(
    BuildContext context,
    WidgetRef ref, {
    DialogTopicDefinition? initial,
  }) async {
    final state = ref.read(dialogTopicsProvider);
    final otherTopics = [
      for (final topic in state.entries)
        if (topic.key != initial?.key) topic,
    ];
    final existingIds = <String>{for (final topic in otherTopics) topic.key};
    final topic = await showDialog<DialogTopicDefinition>(
      context: context,
      builder: (_) => _DialogTopicEditorDialog(
        initial: initial,
        existingIds: existingIds,
        existingTopicClasses: {
          for (final topic in otherTopics) topic.topicClass.toLowerCase(),
        },
        existingSentinelClasses: {
          for (final topic in otherTopics) topic.sentinelClass.toLowerCase(),
        },
      ),
    );
    if (topic == null || !context.mounted) return;

    final notifier = ref.read(dialogTopicsProvider.notifier);
    if (initial == null) {
      notifier.setTopic(topic);
    } else {
      notifier.replaceTopic(initial.id, topic);
    }
  }

  Future<void> _confirmDelete(
    BuildContext context,
    WidgetRef ref,
    DialogTopicDefinition topic,
  ) async {
    final remove = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Delete runtime dialog topic?'),
        content: Text('Remove "${topic.id}" from this mod project?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (remove == true && context.mounted) {
      ref.read(dialogTopicsProvider.notifier).remove(topic.id);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final topics = ref.watch(dialogTopicsProvider).entries;
    final theme = Theme.of(context);

    return ExpansionTile(
      key: PageStorageKey<String>('runtime-dialog-topics-$storageScope'),
      title: Text('Runtime dialog topics (${topics.length})'),
      subtitle: const Text(
        'Explicit participant, topic class, and sentinel class',
      ),
      children: [
        if (topics.isEmpty)
          const Padding(
            padding: EdgeInsets.fromLTRB(16, 4, 16, 8),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text('No runtime dialog topics staged.'),
            ),
          )
        else
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 240),
            child: ListView.builder(
              key: PageStorageKey<String>(
                'runtime-dialog-topic-list-$storageScope',
              ),
              primary: false,
              shrinkWrap: true,
              itemCount: topics.length,
              itemBuilder: (context, index) {
                final topic = topics[index];
                return ListTile(
                  key: ValueKey<String>('runtime-dialog-topic-${topic.key}'),
                  dense: true,
                  leading: CircleAvatar(
                    radius: 13,
                    backgroundColor: theme.colorScheme.secondaryContainer,
                    child: Text('${index + 1}'),
                  ),
                  title: Text(topic.id),
                  subtitle: Text(
                    '${topic.participantName}\n'
                    '${topic.topicClass}  ->  ${topic.sentinelClass}',
                  ),
                  isThreeLine: true,
                  trailing: Wrap(
                    spacing: 4,
                    children: [
                      if (topic.allowHidden)
                        const Tooltip(
                          message: 'Topic may be hidden in its current state',
                          child: Padding(
                            padding: EdgeInsets.all(8),
                            child: Icon(Icons.visibility_off_outlined),
                          ),
                        ),
                      IconButton(
                        icon: const Icon(Icons.edit_outlined),
                        tooltip: 'Edit runtime dialog topic ${topic.id}',
                        onPressed: () =>
                            _openEditor(context, ref, initial: topic),
                      ),
                      IconButton(
                        icon: const Icon(Icons.delete_outline),
                        tooltip: 'Delete runtime dialog topic ${topic.id}',
                        onPressed: () => _confirmDelete(context, ref, topic),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
          child: Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: topics.length >= 64
                  ? null
                  : () => _openEditor(context, ref),
              icon: const Icon(Icons.add, size: 18),
              label: const Text('Add runtime topic'),
            ),
          ),
        ),
      ],
    );
  }
}

class _DialogTopicEditorDialog extends StatefulWidget {
  const _DialogTopicEditorDialog({
    required this.existingIds,
    required this.existingTopicClasses,
    required this.existingSentinelClasses,
    this.initial,
  });

  final DialogTopicDefinition? initial;
  final Set<String> existingIds;
  final Set<String> existingTopicClasses;
  final Set<String> existingSentinelClasses;

  @override
  State<_DialogTopicEditorDialog> createState() =>
      _DialogTopicEditorDialogState();
}

class _DialogTopicEditorDialogState extends State<_DialogTopicEditorDialog> {
  late final TextEditingController _idController;
  late final TextEditingController _participantController;
  late final TextEditingController _topicClassController;
  late final TextEditingController _sentinelClassController;
  late bool _allowHidden;
  String? _error;

  @override
  void initState() {
    super.initState();
    final initial = widget.initial;
    _idController = TextEditingController(text: initial?.id ?? '');
    _participantController = TextEditingController(
      text: initial?.participantName ?? '',
    );
    _topicClassController = TextEditingController(
      text: initial?.topicClass ?? '',
    );
    _sentinelClassController = TextEditingController(
      text: initial?.sentinelClass ?? '',
    );
    _allowHidden = initial?.allowHidden ?? false;
  }

  @override
  void dispose() {
    _idController.dispose();
    _participantController.dispose();
    _topicClassController.dispose();
    _sentinelClassController.dispose();
    super.dispose();
  }

  void _submit() {
    final id = _idController.text.trim();
    final participant = _participantController.text.trim();
    final topicClass = _topicClassController.text.trim();
    final sentinelClass = _sentinelClassController.text.trim();

    final topicClassKey = topicClass.toLowerCase();
    final sentinelClassKey = sentinelClass.toLowerCase();
    final classPathPattern = RegExp(
      r'^/Script/Angelscript\.[A-Za-z_][A-Za-z0-9_]*$',
    );
    final message = id.isEmpty
        ? 'Enter a topic ID.'
        : utf8.encode(id).length > 128 ||
              id.runes.any(
                (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
              )
        ? 'Topic ID must fit in 128 UTF-8 bytes and contain no control characters.'
        : widget.existingIds.contains(id.toLowerCase())
        ? 'This topic ID already exists.'
        : participant.isEmpty ||
              utf8.encode(participant).length > 128 ||
              !RegExp(r'^[A-Za-z0-9_]+$').hasMatch(participant)
        ? 'Participant name must use 1-128 ASCII letters, digits, or underscores.'
        : utf8.encode(topicClass).length > 256 ||
              !classPathPattern.hasMatch(topicClass)
        ? 'Topic class must be an exact /Script/Angelscript.ClassName path.'
        : utf8.encode(sentinelClass).length > 256 ||
              !classPathPattern.hasMatch(sentinelClass)
        ? 'Sentinel class must be an exact /Script/Angelscript.ClassName path.'
        : topicClassKey == sentinelClassKey
        ? 'Topic class and sentinel class must be different.'
        : widget.existingTopicClasses.contains(topicClassKey)
        ? 'This authored topic class is already registered.'
        : widget.existingSentinelClasses.contains(topicClassKey)
        ? 'An authored topic class cannot also be a sentinel class.'
        : widget.existingTopicClasses.contains(sentinelClassKey)
        ? 'A sentinel class cannot be another authored topic class.'
        : null;
    if (message != null) {
      setState(() => _error = message);
      return;
    }

    Navigator.pop(
      context,
      DialogTopicDefinition(
        id: id,
        participantName: participant,
        topicClass: topicClass,
        sentinelClass: sentinelClass,
        allowHidden: _allowHidden,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final editing = widget.initial != null;
    return AlertDialog(
      title: Text(
        editing ? 'Edit runtime dialog topic' : 'Add runtime dialog topic',
      ),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (_error != null) ...[
                Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
                const SizedBox(height: 8),
              ],
              TextField(
                controller: _idController,
                autofocus: true,
                decoration: const InputDecoration(
                  labelText: 'Topic ID',
                  hintText: 'viper_gore_fixture',
                ),
                onChanged: (_) {
                  if (_error != null) setState(() => _error = null);
                },
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _participantController,
                decoration: const InputDecoration(
                  labelText: 'Participant name',
                  hintText: 'om_stt_viper_302',
                ),
                onChanged: (_) {
                  if (_error != null) setState(() => _error = null);
                },
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _topicClassController,
                decoration: const InputDecoration(
                  labelText: 'Topic class',
                  hintText: '/Script/Angelscript.ChoiceGoreViperTopic',
                ),
                onChanged: (_) {
                  if (_error != null) setState(() => _error = null);
                },
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _sentinelClassController,
                decoration: const InputDecoration(
                  labelText: 'Sentinel class',
                  hintText: '/Script/Angelscript.ChoiceStt302ViperExit',
                ),
                onChanged: (_) {
                  if (_error != null) setState(() => _error = null);
                },
                onSubmitted: (_) => _submit(),
              ),
              const SizedBox(height: 12),
              CheckboxListTile(
                key: const ValueKey('dialog-topic-allow-hidden'),
                contentPadding: EdgeInsets.zero,
                value: _allowHidden,
                onChanged: (value) {
                  setState(() => _allowHidden = value ?? false);
                },
                title: const Text('Allow topic to be hidden in this state'),
                subtitle: const Text(
                  'For state-dependent AngelScript visibility. A clean '
                  'zero-match is reported as HIDDEN instead of a failure; '
                  'visible matches still require exact identity.',
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(onPressed: _submit, child: Text(editing ? 'Save' : 'Add')),
      ],
    );
  }
}

class _DialogBrowser extends ConsumerStatefulWidget {
  const _DialogBrowser({this.onlyIds});

  /// See [DialogeTab.onlyIds].
  final Set<String>? onlyIds;

  @override
  ConsumerState<_DialogBrowser> createState() => _DialogBrowserState();
}

/// Sidebar display label for a raw speaker token (`aaron` -> `Aaron`).
String _speakerLabel(String speaker) {
  if (speaker.isEmpty) return '(unknown)';
  return speaker[0].toUpperCase() + speaker.substring(1);
}

class _NewDialogLine {
  const _NewDialogLine(this.id, this.text);

  final String id;
  final String text;
}

class _AddDialogLineDialog extends StatefulWidget {
  const _AddDialogLineDialog({required this.existingIds});

  final Set<String> existingIds;

  @override
  State<_AddDialogLineDialog> createState() => _AddDialogLineDialogState();
}

class _AddDialogLineDialogState extends State<_AddDialogLineDialog> {
  final TextEditingController _idController = TextEditingController();
  final TextEditingController _textController = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _idController.dispose();
    _textController.dispose();
    super.dispose();
  }

  void _submit() {
    final id = _idController.text.trim().toLowerCase();
    final text = _textController.text.trim();
    final validId = RegExp(
      r'^(info|dia|gvl|svm)_[a-z0-9][a-z0-9_]*$',
    ).hasMatch(id);
    final message = !validId
        ? 'Use info_, dia_, gvl_, or svm_ followed by letters, digits, and underscores.'
        : widget.existingIds.contains(id)
        ? 'This localization ID already exists.'
        : text.isEmpty
        ? 'Enter text for the current language.'
        : null;
    if (message != null) {
      setState(() => _error = message);
      return;
    }
    Navigator.pop(context, _NewDialogLine(id, text));
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add localized dialog line'),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: _idController,
              autofocus: true,
              decoration: InputDecoration(
                labelText: 'Localization ID',
                hintText: 'info_viper_gore_01',
                errorText: _error,
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _textController,
              minLines: 2,
              maxLines: 5,
              decoration: const InputDecoration(
                labelText: 'Text in the current game language',
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(onPressed: _submit, child: const Text('Add')),
      ],
    );
  }
}

class _DialogBrowserState extends ConsumerState<_DialogBrowser> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  /// Memoizes the filtered row build (see [DialogRowsMemo]) so per-keystroke
  /// rebuilds while editing inside the Changes tab don't re-scan the catalog.
  final DialogRowsMemo _rowsMemo = DialogRowsMemo();

  /// Stable-identity set of staged dialog IDs absent from the extracted catalog. Text edits emit
  /// a new LocEditsState on every keystroke, but row grouping only needs rebuilding when this key
  /// set actually changes.
  Set<String> _additionalIds = const {};

  /// Key of the speaker group shown in the line list while not searching
  /// (see [DialogGroupRow.groupKey]). Null / vanished keys fall back to the
  /// selected line's group, then to the first group.
  String? _selectedGroupKey;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Set<String> _stableAdditionalIds(
    Map<String, Map<String, String>> catalog,
    Iterable<String> editedIds,
  ) {
    final next = <String>{
      for (final id in editedIds)
        if (isDialogLocId(id.toLowerCase()) &&
            !catalog.containsKey(id.toLowerCase()))
          id.toLowerCase(),
    };
    if (!setEquals(next, _additionalIds)) {
      _additionalIds = Set.unmodifiable(next);
    }
    return _additionalIds;
  }

  Future<void> _addDialogLine() async {
    final catalog = ref.read(locCatalogProvider).value ?? const {};
    final staged = ref.read(locEditsProvider).edits;
    final existingIds = Set<String>.unmodifiable({
      ...catalog.keys.map((id) => id.toLowerCase()),
      ...staged.keys.map((id) => id.toLowerCase()),
    });
    final line = await showDialog<_NewDialogLine>(
      context: context,
      builder: (_) => _AddDialogLineDialog(existingIds: existingIds),
    );
    if (line == null || !mounted) return;

    final lang = gameLangByCode(ref.read(localeProvider));
    ref
        .read(locEditsProvider.notifier)
        .setEdit(line.id, lang.locSets.first, line.text);
    ref.read(selectedDialogIdProvider.notifier).state = line.id;
    setState(() => _selectedGroupKey = null);
  }

  /// Whether [id]'s catalog entry matches [query] by id substring or by any of
  /// its set values containing the query (both already lowercased).
  bool _matches(
    String id,
    String query,
    Map<String, Map<String, String>> catalog,
    Map<String, Map<String, String>> edits,
  ) {
    if (id.contains(query)) return true;
    for (final v in catalog[id]?.values ?? const <String>[]) {
      if (v.toLowerCase().contains(query)) return true;
    }
    // Also match staged edits, so search agrees with the (edited) subtitles + what deploys.
    for (final v in edits[id]?.values ?? const <String>[]) {
      if (v.toLowerCase().contains(query)) return true;
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final catalog = ref.watch(locCatalogProvider).value ?? const {};
    final editedIds = ref.watch(locEditsProvider).edits;
    final additionalIds = _stableAdditionalIds(catalog, editedIds.keys);
    // Filtered views derive their rows locally (memoized on input identity)
    // so the shared unfiltered provider path stays untouched (and
    // un-invalidated) for the main tab.
    final onlyIds = widget.onlyIds;
    final allRows = _rowsMemo.rowsFor(
      catalog,
      onlyIds,
      additionalIds: onlyIds ?? additionalIds,
    );
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;

    final groups = allRows.whereType<DialogGroupRow>().toList();
    final lines = allRows.whereType<DialogLineRow>();
    final selectedId = ref.watch(selectedDialogIdProvider);

    // Resolve selected group. When the stored selection is null or vanished,
    // restore from the still-selected editor line's group (this widget's state
    // dies on tab switches, the id provider doesn't; also covers clearing a
    // search after picking a hit), then fall back to the first group.
    var selectedKey = _selectedGroupKey;
    if (!groups.any((g) => g.groupKey == selectedKey)) {
      selectedKey =
          lines.firstWhereOrNull((l) => l.id == selectedId)?.groupKey ??
          (groups.isEmpty ? null : groups.first.groupKey);
    }

    // Searching: flat cross-group hit list. Otherwise: the selected group's lines.
    final shownLines = searching
        ? lines.where((l) => _matches(l.id, query, catalog, editedIds)).toList()
        : lines.where((l) => l.groupKey == selectedKey).toList();

    final lang = gameLangByCode(ref.watch(localeProvider));

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _searchController,
                  decoration: InputDecoration(
                    labelText: 'Search dialog (id or text)',
                    prefixIcon: const Icon(Icons.search),
                    isDense: true,
                    suffixIcon: _query.isEmpty
                        ? null
                        : IconButton(
                            icon: const Icon(Icons.clear),
                            tooltip: 'Clear',
                            onPressed: () {
                              _searchController.clear();
                              setState(() => _query = '');
                            },
                          ),
                  ),
                  onChanged: (v) => setState(() => _query = v),
                ),
              ),
              if (onlyIds == null) ...[
                const SizedBox(width: 8),
                IconButton.filledTonal(
                  onPressed: _addDialogLine,
                  icon: const Icon(Icons.add),
                  tooltip: 'Add localized dialog line',
                ),
              ],
            ],
          ),
        ),
        Expanded(
          child: groups.isEmpty
              ? const Center(child: Text('No dialog lines match'))
              : Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!searching)
                      SizedBox(
                        width: 230,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerLow,
                          ),
                          child: ListView(
                            padding: const EdgeInsets.symmetric(vertical: 6),
                            children: [
                              for (final g in groups)
                                SidebarTile(
                                  icon: g.isBark
                                      ? Icons.campaign_outlined
                                      : Icons.forum_outlined,
                                  label: l10n.categoryWithCount(
                                    _speakerLabel(g.speaker),
                                    g.lineCount,
                                  ),
                                  selected: g.groupKey == selectedKey,
                                  onTap: () {
                                    // Don't leave a line from ANOTHER group
                                    // open in the editor pane: clear the
                                    // selection unless it belongs to the
                                    // tapped group. Only in the main tab
                                    // (onlyIds == null): a filtered embed does
                                    // NOT own the shared provider — the shared
                                    // selection may be an out-of-filter line
                                    // from the main tab, and the editor's
                                    // out-of-filter guard already shows the
                                    // placeholder, so clearing here would wipe
                                    // the main tab's selection.
                                    final selLine = selectedId == null
                                        ? null
                                        : lines.firstWhereOrNull(
                                            (l) => l.id == selectedId,
                                          );
                                    if (onlyIds == null &&
                                        selectedId != null &&
                                        selLine?.groupKey != g.groupKey) {
                                      ref
                                              .read(
                                                selectedDialogIdProvider
                                                    .notifier,
                                              )
                                              .state =
                                          null;
                                    }
                                    setState(() {
                                      _selectedGroupKey = g.groupKey;
                                    });
                                  },
                                ),
                            ],
                          ),
                        ),
                      ),
                    // The divider belongs to the sidebar — hide both during a
                    // search (matching the audio SFX split view).
                    if (!searching) const VerticalDivider(width: 1),
                    Expanded(
                      child: shownLines.isEmpty
                          ? const Center(child: Text('No dialog lines match'))
                          : ListView.builder(
                              // Reset scroll to top when the shown collection
                              // changes identity (group switch, search toggle).
                              key: searching
                                  ? const ValueKey('search')
                                  : ValueKey(selectedKey),
                              itemCount: shownLines.length,
                              itemBuilder: (context, index) {
                                final line = shownLines[index];
                                // Match the editor field exactly: the staged edit, else the value
                                // in THIS language's target set — with NO English fallback. So the
                                // list preview shows what editing/deploy actually change; a line
                                // empty in the current language shows no preview (like its editor
                                // field), instead of misleading English copy.
                                final set = primarySetFor(
                                  catalog,
                                  line.id,
                                  lang,
                                );
                                final stagedText = editedIds[line.id]?[set];
                                final langValue =
                                    catalog[line.id.toLowerCase()]?[set];
                                final preview =
                                    stagedText ??
                                    ((langValue != null &&
                                            langValue.trim().isNotEmpty)
                                        ? langValue
                                        : null);
                                return ListTile(
                                  dense: true,
                                  selected: line.id == selectedId,
                                  selectedTileColor:
                                      theme.colorScheme.primaryContainer,
                                  leading: editedIds.containsKey(line.id)
                                      ? Icon(
                                          Icons.circle,
                                          size: 10,
                                          color: theme.colorScheme.primary,
                                        )
                                      : const SizedBox(width: 10),
                                  title: Text(
                                    line.id,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                  subtitle: preview == null
                                      ? null
                                      : Text(
                                          preview,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                  onTap: () {
                                    ref
                                        .read(selectedDialogIdProvider.notifier)
                                        .state = line
                                        .id;
                                    // Keep the sidebar in sync with the picked
                                    // line, so clearing a search lands on its
                                    // group (no-op in group view).
                                    setState(
                                      () => _selectedGroupKey = line.groupKey,
                                    );
                                  },
                                );
                              },
                            ),
                    ),
                  ],
                ),
        ),
      ],
    );
  }
}

class _DialogEditor extends ConsumerWidget {
  const _DialogEditor({this.onlyIds});

  /// See [DialogeTab.onlyIds].
  final Set<String>? onlyIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final selectedId = ref.watch(selectedDialogIdProvider);
    // View-level guard for filtered embeddings (the Changes tab): the shared
    // selection may point at a line OUTSIDE the filter — picked on the main
    // Dialoge tab, or its last staged edit was just removed. Show the
    // placeholder instead of an out-of-filter editor, but do NOT clear the
    // shared provider: the main tab owns that selection and keeps it.
    final filter = onlyIds;
    final id =
        (filter != null && selectedId != null && !filter.contains(selectedId))
        ? null
        : selectedId;
    if (id == null) {
      return Center(
        child: Text(
          'Select a dialog line to edit',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.translate, size: 20),
            const SizedBox(width: 8),
            Expanded(child: Text(id, style: theme.textTheme.titleMedium)),
            TextButton.icon(
              onPressed: () =>
                  ref.read(locEditsProvider.notifier).clearForId(id),
              icon: const Icon(Icons.undo, size: 18),
              label: const Text('Clear edits for this line'),
            ),
          ],
        ),
        const SizedBox(height: 12),
        // Filtered embeddings (the Changes tab) review staged edits, so show
        // only the languages that actually carry one; the main tab shows all.
        LangFieldsEditor(locId: id, onlyEdited: onlyIds != null),
        VoiceLineEditor(key: ValueKey('voice-$id'), locId: id),
      ],
    );
  }
}
