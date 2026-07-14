import 'package:flutter/material.dart';

import 'revision3_content_index.dart';
import 'revision3_quest_outline_authoring.dart';

/// Count-preserving editor for the visible outline of one exact managed-R3
/// QuestDraft. It intentionally exposes no technical identity or runtime
/// controls.
class Revision3QuestOutlineEditDialog extends StatefulWidget {
  const Revision3QuestOutlineEditDialog({
    required this.index,
    required this.quest,
    required this.publish,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final Revision3QuestOutlineEditPublisher publish;

  @override
  State<Revision3QuestOutlineEditDialog> createState() =>
      _Revision3QuestOutlineEditDialogState();
}

class _Revision3QuestOutlineEditDialogState
    extends State<Revision3QuestOutlineEditDialog> {
  late final TextEditingController _displayName;
  late final TextEditingController _title;
  late final List<TextEditingController> _objectives;
  String? _error;
  bool _busy = false;
  bool _checkpointLocked = false;

  Revision3ContentQuestDraftSummary get _summary =>
      widget.quest.summary.questDraft!;

  @override
  void initState() {
    super.initState();
    final summary = _summary;
    _displayName = TextEditingController(text: widget.quest.displayName);
    _title = TextEditingController(text: summary.title);
    _objectives = [
      for (final objective in summary.objectiveTitles)
        TextEditingController(text: objective),
    ];
    _displayName.addListener(_fieldChanged);
    _title.addListener(_fieldChanged);
    for (final controller in _objectives) {
      controller.addListener(_fieldChanged);
    }
  }

  @override
  void dispose() {
    _displayName.removeListener(_fieldChanged);
    _title.removeListener(_fieldChanged);
    _displayName.dispose();
    _title.dispose();
    for (final controller in _objectives) {
      controller.removeListener(_fieldChanged);
      controller.dispose();
    }
    super.dispose();
  }

  void _fieldChanged() {
    if (mounted) setState(() => _error = null);
  }

  bool get _hasChanges {
    if (_displayName.text != widget.quest.displayName ||
        _title.text != _summary.title) {
      return true;
    }
    for (var index = 0; index < _objectives.length; index++) {
      if (_objectives[index].text != _summary.objectiveTitles[index]) {
        return true;
      }
    }
    return false;
  }

  void _move(int from, int to) {
    if (_busy || _checkpointLocked || to < 0 || to >= _objectives.length) {
      return;
    }
    setState(() {
      final item = _objectives.removeAt(from);
      _objectives.insert(to, item);
      _error = null;
    });
  }

  Future<void> _save() async {
    if (_busy || _checkpointLocked) return;
    final objectiveTitles = [
      for (final controller in _objectives) controller.text,
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
      input = Revision3QuestOutlineEditInput.forQuest(
        index: widget.index,
        quest: widget.quest,
        displayName: _displayName.text,
        title: _title.text,
        objectiveTitles: objectiveTitles,
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
      Navigator.of(context).pop(publication);
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
    return AlertDialog(
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
                'Rename the library entry, Quest title, or existing objectives. Objective count, technical identity, module, parent and giver stay unchanged.',
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
              for (var index = 0; index < _objectives.length; index++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          key: Key('revision3-quest-outline-objective-$index'),
                          controller: _objectives[index],
                          enabled: !_busy && !_checkpointLocked,
                          decoration: InputDecoration(
                            labelText: 'Objective ${index + 1}',
                          ),
                        ),
                      ),
                      IconButton(
                        key: Key('revision3-quest-outline-objective-up-$index'),
                        tooltip: 'Move objective up',
                        onPressed: !_busy && !_checkpointLocked && index > 0
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
                            !_busy &&
                                !_checkpointLocked &&
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
                '${summary.objectiveTitles.length} existing objective${summary.objectiveTitles.length == 1 ? '' : 's'}. Objective count and Quest relationships stay unchanged.',
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
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('revision3-quest-outline-cancel'),
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton.icon(
          key: const Key('revision3-quest-outline-save'),
          onPressed: _busy || _checkpointLocked || !_hasChanges ? null : _save,
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
  }
}
