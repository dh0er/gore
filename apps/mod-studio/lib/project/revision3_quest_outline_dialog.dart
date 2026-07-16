import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_quest_outline_authoring.dart';
import 'revision3_quest_transitions_authoring.dart';

/// Count-preserving editor for the visible outline of one exact managed-R3
/// QuestDraft. It intentionally exposes no technical identity or runtime
/// controls.
class Revision3QuestOutlineEditDialog extends StatefulWidget {
  const Revision3QuestOutlineEditDialog({
    required this.index,
    required this.quest,
    required this.publish,
    this.loadTransitionSeed,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final Revision3QuestOutlineEditPublisher publish;
  final Revision3QuestTransitionsSeedLoader? loadTransitionSeed;

  @override
  State<Revision3QuestOutlineEditDialog> createState() =>
      _Revision3QuestOutlineEditDialogState();
}

class _Revision3QuestOutlineEditDialogState
    extends State<Revision3QuestOutlineEditDialog> {
  late final TextEditingController _displayName;
  late final TextEditingController _title;
  late final List<_Revision3QuestOutlineObjectiveField> _objectives;
  AuthoringRevision3QuestTransitionsSeed? _transitionSeed;
  String? _error;
  late bool _loading;
  bool _busy = false;
  bool _checkpointLocked = false;
  bool _allowPop = false;
  bool _confirmingDiscard = false;
  int _loadGeneration = 0;

  Revision3ContentQuestDraftSummary get _summary =>
      widget.quest.summary.questDraft!;

  bool get _usesStableObjectiveSlots =>
      _transitionSeed?.legacySynthetic == false;

  bool get _objectiveEditingEnabled =>
      !_loading &&
      !_busy &&
      !_checkpointLocked &&
      (widget.loadTransitionSeed == null || _transitionSeed != null);

  @override
  void initState() {
    super.initState();
    final summary = _summary;
    _displayName = TextEditingController(text: widget.quest.displayName);
    _title = TextEditingController(text: summary.title);
    _objectives = <_Revision3QuestOutlineObjectiveField>[
      for (var index = 0; index < summary.objectiveTitles.length; index++)
        _Revision3QuestOutlineObjectiveField(
          slot: index + 1,
          controller: TextEditingController(
            text: summary.objectiveTitles[index],
          ),
        ),
    ];
    _loading = widget.loadTransitionSeed != null;
    _displayName.addListener(_fieldChanged);
    _title.addListener(_fieldChanged);
    for (final objective in _objectives) {
      objective.controller.addListener(_fieldChanged);
    }
    if (_loading) _loadTransitionSeed();
  }

  @override
  void dispose() {
    _displayName.removeListener(_fieldChanged);
    _title.removeListener(_fieldChanged);
    _displayName.dispose();
    _title.dispose();
    _loadGeneration++;
    for (final objective in _objectives) {
      objective.controller.removeListener(_fieldChanged);
      objective.controller.dispose();
    }
    super.dispose();
  }

  void _fieldChanged() {
    if (mounted) setState(() => _error = null);
  }

  Future<void> _loadTransitionSeed() async {
    final loader = widget.loadTransitionSeed;
    if (loader == null) return;
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final moduleReferences = widget.quest.references
          .where(
            (reference) =>
                reference.role == 'draft_script_module' &&
                reference.qualifier == null &&
                reference.resolution ==
                    Revision3ContentReferenceResolution.resolved &&
                reference.target.projectId == widget.index.projectId &&
                reference.target.expectedKind ==
                    Revision3ContentEntityKind.scriptModule,
          )
          .toList(growable: false);
      if (moduleReferences.length != 1) {
        throw const FormatException(
          'The selected Quest does not own exactly one generated script.',
        );
      }
      final module = widget.index.entityById(
        moduleReferences.single.target.entityId,
      );
      if (module == null ||
          module.kind != Revision3ContentEntityKind.scriptModule) {
        throw const FormatException(
          'The selected Quest script is not available in this project view.',
        );
      }
      final seed = await loader(
        questId: widget.quest.id,
        expectedQuestRevision: widget.quest.revision,
        expectedModuleId: module.id,
        expectedModuleRevision: module.revision,
      );
      final seedTitles = [
        for (final objective in seed.objectives) objective.title,
      ];
      if (seedTitles.length != _summary.objectiveTitles.length ||
          !_sameStrings(seedTitles, _summary.objectiveTitles)) {
        throw const FormatException(
          'The Quest behavior seed disagrees with the visible objectives.',
        );
      }
      Revision3QuestOutlineEditInput.forQuestWithTransitionSeed(
        index: widget.index,
        quest: widget.quest,
        seed: seed,
        displayName: widget.quest.displayName,
        title: _summary.title,
        objectives: [
          for (final objective in seed.objectives)
            Revision3QuestOutlineObjectiveEdit(
              slot: objective.slot,
              title: objective.title,
            ),
        ],
      );
      if (!mounted || generation != _loadGeneration) return;
      for (final objective in _objectives) {
        objective.controller.removeListener(_fieldChanged);
        objective.controller.dispose();
      }
      setState(() {
        _objectives
          ..clear()
          ..addAll([
            for (final objective in seed.objectives)
              _Revision3QuestOutlineObjectiveField(
                slot: objective.slot,
                controller: TextEditingController(text: objective.title)
                  ..addListener(_fieldChanged),
              ),
          ]);
        _transitionSeed = seed;
        _loading = false;
      });
    } on Revision3QuestTransitionsStaleCheckpointException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _checkpointLocked = true;
        _error =
            'The project changed while this editor opened. Close it and reopen the Quest from the refreshed library.';
      });
    } on Revision3QuestTransitionsRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _checkpointLocked = true;
        _error =
            'The project checkpoint can no longer be verified. Close this editor and reopen the managed project.';
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _error =
            'Quest objective identities could not be loaded. No project or game files were changed.';
      });
    }
  }

  bool get _hasChanges {
    if (_displayName.text != widget.quest.displayName ||
        _title.text != _summary.title) {
      return true;
    }
    final seed = _transitionSeed;
    for (var index = 0; index < _objectives.length; index++) {
      final expectedSlot = seed?.objectives[index].slot ?? index + 1;
      final expectedTitle =
          seed?.objectives[index].title ?? _summary.objectiveTitles[index];
      if (_objectives[index].slot != expectedSlot ||
          _objectives[index].controller.text != expectedTitle) {
        return true;
      }
    }
    return false;
  }

  void _move(int from, int to) {
    if (_loading ||
        _busy ||
        _checkpointLocked ||
        to < 0 ||
        to >= _objectives.length) {
      return;
    }
    setState(() {
      final item = _objectives.removeAt(from);
      _objectives.insert(to, item);
      _error = null;
    });
  }

  Future<void> _save() async {
    if (_loading ||
        _busy ||
        _checkpointLocked ||
        (widget.loadTransitionSeed != null && _transitionSeed == null)) {
      return;
    }
    final objectiveTitles = [
      for (final objective in _objectives) objective.controller.text,
    ];
    final problem = Revision3QuestOutlineEditInput.validateFields(
      displayName: _displayName.text,
      title: _title.text,
      objectiveTitles: objectiveTitles,
    );
    if (problem != null) {
      setState(() => _error = problem);
      return;
    }
    final Revision3QuestOutlineEditInput input;
    try {
      final seed = _transitionSeed;
      input = seed == null
          ? Revision3QuestOutlineEditInput.forQuest(
              index: widget.index,
              quest: widget.quest,
              displayName: _displayName.text,
              title: _title.text,
              objectiveTitles: objectiveTitles,
            )
          : Revision3QuestOutlineEditInput.forQuestWithTransitionSeed(
              index: widget.index,
              quest: widget.quest,
              seed: seed,
              displayName: _displayName.text,
              title: _title.text,
              objectives: [
                for (final objective in _objectives)
                  Revision3QuestOutlineObjectiveEdit(
                    slot: objective.slot,
                    title: objective.controller.text,
                  ),
              ],
            );
    } on FormatException catch (error) {
      setState(() => _error = error.message);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.publish(input: input);
      if (!mounted) return;
      await _popAfterUnlock(publication);
    } on Revision3QuestOutlineStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project changed while this editor was open. Close it and reopen the Quest from the refreshed library.';
      });
    } on Revision3QuestOutlineRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project checkpoint can no longer be verified. Close this editor and reopen the managed project before editing.';
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'The Quest outline could not be saved. Nothing was published; check the fields and try again.';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final summary = _summary;
    final dialog = AlertDialog(
      key: const Key('revision3-quest-outline-dialog'),
      title: const Text('Edit quest outline'),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                _usesStableObjectiveSlots
                    ? 'Rename the library entry, Quest title, or existing objectives. Reordering keeps each objective identity and its behavior connections intact.'
                    : 'Rename the library entry, Quest title, or existing objectives. Objective count and Quest relationships stay unchanged.',
                style: Theme.of(context).textTheme.bodyMedium,
              ),
              const SizedBox(height: 12),
              Container(
                key: const Key('revision3-quest-outline-boundary'),
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.secondaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Text(
                  'This saves an offline project draft only. Build remains blocked, runtime behavior remains unqualified, and nothing is published or deployed to the game.',
                ),
              ),
              const SizedBox(height: 16),
              TextField(
                key: const Key('revision3-quest-outline-display-name'),
                controller: _displayName,
                enabled: !_busy && !_checkpointLocked,
                decoration: const InputDecoration(
                  labelText: 'Name in project library',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                key: const Key('revision3-quest-outline-title'),
                controller: _title,
                enabled: !_busy && !_checkpointLocked,
                decoration: const InputDecoration(labelText: 'Quest title'),
              ),
              const SizedBox(height: 18),
              Text('Objectives', style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 6),
              if (_loading) ...[
                const LinearProgressIndicator(
                  key: Key('revision3-quest-outline-loading-identities'),
                ),
                const SizedBox(height: 10),
                const Text('Loading stable objective identities…'),
                const SizedBox(height: 10),
              ],
              for (var index = 0; index < _objectives.length; index++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          key: Key('revision3-quest-outline-objective-$index'),
                          controller: _objectives[index].controller,
                          enabled: _objectiveEditingEnabled,
                          decoration: InputDecoration(
                            labelText: 'Objective ${index + 1}',
                          ),
                        ),
                      ),
                      IconButton(
                        key: Key('revision3-quest-outline-objective-up-$index'),
                        tooltip: 'Move objective up',
                        onPressed: _objectiveEditingEnabled && index > 0
                            ? () => _move(index, index - 1)
                            : null,
                        icon: const Icon(Icons.arrow_upward),
                      ),
                      IconButton(
                        key: Key(
                          'revision3-quest-outline-objective-down-$index',
                        ),
                        tooltip: 'Move objective down',
                        onPressed:
                            _objectiveEditingEnabled &&
                                index + 1 < _objectives.length
                            ? () => _move(index, index + 1)
                            : null,
                        icon: const Icon(Icons.arrow_downward),
                      ),
                    ],
                  ),
                ),
              const SizedBox(height: 6),
              Text(
                '${summary.objectiveTitles.length} existing objective${summary.objectiveTitles.length == 1 ? '' : 's'}. ${_usesStableObjectiveSlots ? 'Objective count, stable IDs and Quest relationships stay unchanged.' : 'Objective count and Quest relationships stay unchanged.'}',
                key: const Key('revision3-quest-outline-fixed-context'),
                style: Theme.of(context).textTheme.bodySmall,
              ),
              if (_error != null) ...[
                const SizedBox(height: 14),
                Semantics(
                  liveRegion: true,
                  child: Text(
                    _error!,
                    key: const Key('revision3-quest-outline-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              ],
              if (!_loading &&
                  widget.loadTransitionSeed != null &&
                  _transitionSeed == null &&
                  !_checkpointLocked) ...[
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  key: const Key('revision3-quest-outline-retry-identities'),
                  onPressed: _busy ? null : _loadTransitionSeed,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Retry loading objectives'),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('revision3-quest-outline-cancel'),
          onPressed: _loading || _busy || _confirmingDiscard
              ? null
              : _requestDismiss,
          child: Text(_checkpointLocked ? 'Close' : 'Cancel'),
        ),
        FilledButton.icon(
          key: const Key('revision3-quest-outline-save'),
          onPressed:
              _loading ||
                  _busy ||
                  _checkpointLocked ||
                  (widget.loadTransitionSeed != null &&
                      _transitionSeed == null) ||
                  !_hasChanges
              ? null
              : _save,
          icon: _busy
              ? const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.save_outlined),
          label: Text(_busy ? 'Saving…' : 'Save outline'),
        ),
      ],
    );
    return PopScope(
      canPop: _allowPop || (!_loading && !_busy && !_hasChanges),
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(_requestDismiss());
      },
      child: dialog,
    );
  }

  Future<void> _requestDismiss() async {
    if (_loading || _busy || _confirmingDiscard) return;
    if (_checkpointLocked) {
      await _popAfterUnlock();
      return;
    }
    if (!_hasChanges) {
      await _popAfterUnlock();
      return;
    }
    setState(() => _confirmingDiscard = true);
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const Key('revision3-quest-outline-discard-dialog'),
        title: const Text('Discard Quest outline changes?'),
        content: const Text(
          'Your unsaved Quest name, title, objective text, and objective order will be lost.',
        ),
        actions: [
          TextButton(
            key: const Key('revision3-quest-outline-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Keep editing'),
          ),
          FilledButton(
            key: const Key('revision3-quest-outline-discard'),
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Discard'),
          ),
        ],
      ),
    );
    if (!mounted) return;
    setState(() => _confirmingDiscard = false);
    if (discard == true) await _popAfterUnlock();
  }

  Future<void> _popAfterUnlock([
    Revision3QuestOutlineEditPublication? result,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(result);
  }
}

final class _Revision3QuestOutlineObjectiveField {
  const _Revision3QuestOutlineObjectiveField({
    required this.slot,
    required this.controller,
  });

  final int slot;
  final TextEditingController controller;
}

bool _sameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
