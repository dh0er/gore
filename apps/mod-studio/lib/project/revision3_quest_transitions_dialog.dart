import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_quest_logic_preview.dart';
import 'revision3_quest_transitions_authoring.dart';

/// Visual, bounded editor for one exact-current Quest behavior plan.
class Revision3QuestTransitionsEditDialog extends StatefulWidget {
  const Revision3QuestTransitionsEditDialog({
    required this.index,
    required this.quest,
    required this.service,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final Revision3QuestTransitionsAuthoringService service;

  @override
  State<Revision3QuestTransitionsEditDialog> createState() =>
      _Revision3QuestTransitionsEditDialogState();
}

class _Revision3QuestTransitionsEditDialogState
    extends State<Revision3QuestTransitionsEditDialog> {
  Revision3QuestTransitionsEditCheckpoint? _checkpoint;
  AuthoringRevision3QuestTransitionPlanV1? _plan;
  String? _error;
  bool _loading = true;
  bool _busy = false;
  bool _checkpointLocked = false;
  bool _allowPop = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _loadGeneration++;
    super.dispose();
  }

  Future<void> _load() async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final checkpoint = await widget.service.load(
        index: widget.index,
        quest: widget.quest,
      );
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _checkpoint = checkpoint;
        _plan = checkpoint.transitionPlan;
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
            'Quest behavior could not be loaded. No project or game files were changed.';
      });
    }
  }

  bool get _hasChanges {
    final checkpoint = _checkpoint;
    final plan = _plan;
    return checkpoint != null &&
        plan != null &&
        checkpoint.transitionPlan.canonicalJson != plan.canonicalJson;
  }

  Future<void> _save() async {
    final checkpoint = _checkpoint;
    final plan = _plan;
    if (_busy ||
        _checkpointLocked ||
        checkpoint == null ||
        plan == null ||
        !_hasChanges) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.service.publish(
        checkpoint: checkpoint,
        transitionPlan: plan,
      );
      if (!mounted) return;
      await _popAfterUnlock(publication);
    } on Revision3QuestTransitionsStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project changed while this editor was open. Close it and reopen the Quest from the refreshed library.';
      });
    } on Revision3QuestTransitionsRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project checkpoint can no longer be verified. Close this editor and reopen the managed project.';
      });
    } on FormatException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = error.message;
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = _transitionErrorMessage(error.code);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'Quest behavior could not be saved safely. Nothing was built, deployed, or written into the game.';
      });
    }
  }

  void _applySequentialTemplate() {
    final plan = _plan;
    if (plan == null || _busy || _checkpointLocked) return;
    try {
      setState(() {
        _plan = Revision3QuestTransitionsAuthoringService.sequentialTemplate(
          plan,
        );
        _error = null;
      });
    } on FormatException catch (error) {
      setState(() => _error = error.message);
    }
  }

  Future<void> _editTransition(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) async {
    final checkpoint = _checkpoint;
    final plan = _plan;
    if (checkpoint == null || plan == null || _busy || _checkpointLocked) {
      return;
    }
    final current = _transitionFor(plan, node, edge);
    final result = await showDialog<_TransitionEditResult>(
      context: context,
      barrierDismissible: false,
      builder: (context) => _TransitionEditorDialog(
        checkpoint: checkpoint,
        plan: plan,
        node: node,
        edge: edge,
        current: current,
      ),
    );
    if (!mounted || result == null) return;
    try {
      final updated = result.remove
          ? Revision3QuestTransitionsAuthoringService.removeOptionalTransition(
              plan,
              node: node,
              edge: edge,
            )
          : Revision3QuestTransitionsAuthoringService.setTransition(
              plan,
              result.transition!,
            );
      setState(() {
        _plan = updated;
        _error = null;
      });
    } on FormatException catch (error) {
      setState(() => _error = error.message);
    }
  }

  Future<void> _openLogicPreview() async {
    final checkpoint = _checkpoint;
    final plan = _plan;
    if (checkpoint == null || plan == null || _busy || _checkpointLocked) {
      return;
    }
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) =>
          _QuestLogicPreviewDialog(checkpoint: checkpoint, plan: plan),
    );
  }

  @override
  Widget build(BuildContext context) {
    final checkpoint = _checkpoint;
    final plan = _plan;
    final enabled = !_loading && !_busy && !_checkpointLocked;
    return PopScope(
      canPop: _allowPop || (!_busy && !_hasChanges),
      child: AlertDialog(
        key: const Key('revision3-quest-transitions-dialog'),
        title: const Text('Edit Quest behavior'),
        content: SizedBox(
          width: 920,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  widget.quest.displayName,
                  key: const Key('revision3-quest-transitions-quest-name'),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                const Text(
                  'Choose what can make the Quest and each objective become available, start, succeed, or fail.',
                ),
                const SizedBox(height: 12),
                Container(
                  key: const Key('revision3-quest-transitions-boundary'),
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Text(
                    'This saves an offline project checkpoint only. It does not build, run, or deploy the Quest, and it does not qualify behavior in the game.',
                  ),
                ),
                const SizedBox(height: 16),
                if (_loading)
                  const Center(
                    child: Padding(
                      padding: EdgeInsets.all(24),
                      child: CircularProgressIndicator(),
                    ),
                  )
                else if (checkpoint != null && plan != null) ...[
                  if (checkpoint.seed.legacySynthetic) ...[
                    const Text(
                      'This Quest still uses its original fixed behavior. Apply a template or configure a cell to create an editable behavior plan.',
                      key: Key('revision3-quest-transitions-legacy-note'),
                    ),
                    const SizedBox(height: 12),
                  ],
                  Align(
                    alignment: Alignment.centerLeft,
                    child: OutlinedButton.icon(
                      key: const Key(
                        'revision3-quest-transitions-sequential-template',
                      ),
                      onPressed: enabled ? _applySequentialTemplate : null,
                      icon: const Icon(Icons.account_tree_outlined),
                      label: const Text('Apply sequential template'),
                    ),
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Engine default means the game may trigger that step directly, with no extra condition or follow-up action.',
                  ),
                  const SizedBox(height: 12),
                  _BehaviorTable(
                    checkpoint: checkpoint,
                    plan: plan,
                    enabled: enabled,
                    onEdit: _editTransition,
                  ),
                  const SizedBox(height: 12),
                  Align(
                    alignment: Alignment.centerLeft,
                    child: OutlinedButton.icon(
                      key: const Key('revision3-quest-logic-preview-open'),
                      onPressed: enabled ? _openLogicPreview : null,
                      icon: const Icon(Icons.play_circle_outline),
                      label: const Text('Preview project logic'),
                    ),
                  ),
                ],
                if (_error != null) ...[
                  const SizedBox(height: 14),
                  Semantics(
                    liveRegion: true,
                    child: Text(
                      _error!,
                      key: const Key('revision3-quest-transitions-error'),
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
          if (!_loading && checkpoint == null && !_checkpointLocked)
            TextButton.icon(
              key: const Key('revision3-quest-transitions-retry'),
              onPressed: _busy ? null : _load,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          TextButton(
            key: const Key('revision3-quest-transitions-cancel'),
            onPressed: _busy ? null : _cancel,
            child: Text(_checkpointLocked ? 'Close' : 'Cancel'),
          ),
          FilledButton.icon(
            key: const Key('revision3-quest-transitions-save'),
            onPressed: enabled && _hasChanges ? _save : null,
            icon: _busy
                ? const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.save_outlined),
            label: Text(_busy ? 'Saving…' : 'Save behavior'),
          ),
        ],
      ),
    );
  }

  Future<void> _cancel() async {
    if (!_hasChanges) {
      await _popAfterUnlock();
      return;
    }
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const Key('revision3-quest-transitions-discard-dialog'),
        title: const Text('Discard behavior changes?'),
        content: const Text('Your unsaved Quest behavior will be lost.'),
        actions: [
          TextButton(
            key: const Key('revision3-quest-transitions-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Keep editing'),
          ),
          FilledButton(
            key: const Key('revision3-quest-transitions-discard'),
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Discard'),
          ),
        ],
      ),
    );
    if (!mounted || discard != true) return;
    await _popAfterUnlock();
  }

  Future<void> _popAfterUnlock([
    Revision3QuestTransitionsEditPublication? result,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(result);
  }
}

final class _QuestLogicPreviewDialog extends StatefulWidget {
  const _QuestLogicPreviewDialog({
    required this.checkpoint,
    required this.plan,
  });

  final Revision3QuestTransitionsEditCheckpoint checkpoint;
  final AuthoringRevision3QuestTransitionPlanV1 plan;

  @override
  State<_QuestLogicPreviewDialog> createState() =>
      _QuestLogicPreviewDialogState();
}

final class _QuestLogicPreviewDialogState
    extends State<_QuestLogicPreviewDialog> {
  late final Revision3QuestLogicPreview _preview;
  String? _notice;

  @override
  void initState() {
    super.initState();
    _preview = Revision3QuestLogicPreview(widget.plan);
  }

  @override
  Widget build(BuildContext context) {
    final outsideModelCount =
        _preview.predicateConjunctionsOutsideExclusiveModel;
    final nodes = <AuthoringRevision3QuestTransitionNodeV1>[
      const AuthoringRevision3QuestTransitionNodeV1.root(),
      for (final slot in widget.plan.objectiveOrder)
        AuthoringRevision3QuestTransitionNodeV1.objective(slot),
    ];
    return AlertDialog(
      key: const Key('revision3-quest-logic-preview-dialog'),
      title: const Text('Preview Quest project logic'),
      content: SizedBox(
        width: 940,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(
                key: const Key('revision3-quest-logic-preview-boundary'),
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.tertiaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Text(
                  'Project logic preview only. It uses five conservative, mutually exclusive offline phases: Unavailable, Available, Running, Succeeded, and Failed. Started and Completed are derived. Generated engine state calls are independent; combinations outside these phases are not represented or proven. It does not run the engine, prove runtime polling or handler order, build, deploy, touch a save, or qualify this Quest in the game.',
                ),
              ),
              if (outsideModelCount > 0) ...[
                const SizedBox(height: 10),
                Container(
                  key: const Key(
                    'revision3-quest-logic-preview-model-boundary',
                  ),
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    '$outsideModelCount condition alternative${outsideModelCount == 1 ? '' : 's'} cannot be represented by the five exclusive preview phases and therefore always evaluate${outsideModelCount == 1 ? 's' : ''} false here. The renderer still emits the independent engine state calls; this is not a runtime verdict.',
                  ),
                ),
              ],
              const SizedBox(height: 16),
              Text(
                'Lifecycle state',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 6),
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: DataTable(
                  key: const Key('revision3-quest-logic-preview-state-table'),
                  columns: const [
                    DataColumn(label: Text('Quest part')),
                    DataColumn(label: Text('Available')),
                    DataColumn(label: Text('Running')),
                    DataColumn(label: Text('Started')),
                    DataColumn(label: Text('Succeeded')),
                    DataColumn(label: Text('Failed')),
                    DataColumn(label: Text('Completed')),
                  ],
                  rows: [
                    for (final node in nodes)
                      DataRow(
                        cells: [
                          DataCell(Text(_nodeLabel(widget.checkpoint, node))),
                          for (final test
                              in AuthoringRevision3QuestTransitionStateTestV1
                                  .values)
                            DataCell(
                              Text(
                                _preview.stateOf(node).matches(test)
                                    ? 'Yes'
                                    : '—',
                                key: Key(
                                  'revision3-quest-logic-preview-state-${node.stableKey}-${test.wireName}',
                                ),
                              ),
                            ),
                        ],
                      ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              Text(
                'External test triggers',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              const Text(
                'Only edges explicitly marked as external in this project plan appear here.',
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final trigger in _preview.externalTriggers)
                    OutlinedButton(
                      key: Key(
                        'revision3-quest-logic-preview-trigger-${trigger.node.stableKey}-${trigger.edge.wireName}',
                      ),
                      onPressed: trigger.enabled
                          ? () => _trigger(trigger.node, trigger.edge)
                          : null,
                      child: Text(
                        '${_nodeLabel(widget.checkpoint, trigger.node)} · ${_edgeLabel(trigger.edge)}',
                      ),
                    ),
                ],
              ),
              if (_notice != null) ...[
                const SizedBox(height: 8),
                Text(
                  _notice!,
                  key: const Key('revision3-quest-logic-preview-notice'),
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              const SizedBox(height: 16),
              Text('Timeline', style: Theme.of(context).textTheme.titleMedium),
              if (_preview.traceWasTrimmed)
                const Text(
                  'Older preview events were removed to keep this timeline bounded.',
                ),
              Container(
                key: const Key('revision3-quest-logic-preview-timeline'),
                constraints: const BoxConstraints(maxHeight: 260),
                child: ListView.builder(
                  shrinkWrap: true,
                  itemCount: _preview.trace.length,
                  itemBuilder: (context, index) {
                    final entry = _preview.trace[index];
                    return ListTile(
                      dense: true,
                      visualDensity: VisualDensity.compact,
                      leading: Text('${entry.sequence}'),
                      title: Text(_traceLabel(entry)),
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton.icon(
          key: const Key('revision3-quest-logic-preview-reset'),
          onPressed: _reset,
          icon: const Icon(Icons.restart_alt),
          label: const Text('Reset preview'),
        ),
        FilledButton(
          key: const Key('revision3-quest-logic-preview-close'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }

  void _trigger(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) {
    final result = _preview.triggerExternal(node, edge);
    setState(() {
      _notice = result.status == Revision3QuestLogicPreviewActionStatus.refused
          ? result.message
          : null;
    });
  }

  void _reset() {
    final result = _preview.reset();
    setState(() {
      _notice = result.status == Revision3QuestLogicPreviewActionStatus.refused
          ? result.message
          : null;
    });
  }

  String _traceLabel(Revision3QuestLogicPreviewTraceEntry entry) {
    final node = _nodeLabel(widget.checkpoint, entry.node);
    final edge = entry.edge == null ? null : _edgeLabel(entry.edge!);
    final source = entry.source == null
        ? null
        : _nodeLabel(widget.checkpoint, entry.source!);
    return switch (entry.kind) {
      Revision3QuestLogicPreviewTraceKind.reset => 'Preview reset.',
      Revision3QuestLogicPreviewTraceKind.external =>
        'External test trigger: $node · $edge.',
      Revision3QuestLogicPreviewTraceKind.predicate =>
        'Automatic condition: $node · $edge.',
      Revision3QuestLogicPreviewTraceKind.effect =>
        '$source follow-up action: $node · $edge.',
      Revision3QuestLogicPreviewTraceKind.parentSuccess =>
        '$source completed its parent: $node · $edge.',
      Revision3QuestLogicPreviewTraceKind.ignored =>
        entry.detail ?? '$node · $edge was skipped.',
      Revision3QuestLogicPreviewTraceKind.refused =>
        entry.detail ?? 'The bounded project preview refused this action.',
    };
  }
}

final class _BehaviorTable extends StatelessWidget {
  const _BehaviorTable({
    required this.checkpoint,
    required this.plan,
    required this.enabled,
    required this.onEdit,
  });

  final Revision3QuestTransitionsEditCheckpoint checkpoint;
  final AuthoringRevision3QuestTransitionPlanV1 plan;
  final bool enabled;
  final Future<void> Function(
    AuthoringRevision3QuestTransitionNodeV1,
    AuthoringRevision3QuestTransitionEdgeV1,
  )
  onEdit;

  @override
  Widget build(BuildContext context) {
    final nodes = <AuthoringRevision3QuestTransitionNodeV1>[
      const AuthoringRevision3QuestTransitionNodeV1.root(),
      for (final slot in plan.objectiveOrder)
        AuthoringRevision3QuestTransitionNodeV1.objective(slot),
    ];
    return Container(
      key: const Key('revision3-quest-transitions-behavior-table'),
      decoration: BoxDecoration(
        border: Border.all(color: Theme.of(context).dividerColor),
        borderRadius: BorderRadius.circular(8),
      ),
      clipBehavior: Clip.antiAlias,
      child: Table(
        columnWidths: const <int, TableColumnWidth>{
          0: FlexColumnWidth(2.2),
          1: FlexColumnWidth(1.2),
          2: FlexColumnWidth(1.2),
          3: FlexColumnWidth(1.2),
          4: FlexColumnWidth(1.2),
        },
        border: TableBorder(
          horizontalInside: BorderSide(color: Theme.of(context).dividerColor),
          verticalInside: BorderSide(color: Theme.of(context).dividerColor),
        ),
        children: <TableRow>[
          TableRow(
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
            ),
            children: const <Widget>[
              _TableLabel('Quest part'),
              _TableLabel('Available'),
              _TableLabel('Start'),
              _TableLabel('Success'),
              _TableLabel('Failure'),
            ],
          ),
          for (final node in nodes)
            TableRow(
              children: <Widget>[
                _TableLabel(_nodeLabel(checkpoint, node)),
                for (final edge
                    in AuthoringRevision3QuestTransitionEdgeV1.values)
                  _TransitionCell(
                    node: node,
                    edge: edge,
                    transition: _transitionFor(plan, node, edge),
                    enabled: enabled,
                    onPressed: () => onEdit(node, edge),
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

final class _TableLabel extends StatelessWidget {
  const _TableLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 12),
    child: Text(label, overflow: TextOverflow.ellipsis),
  );
}

final class _TransitionCell extends StatelessWidget {
  const _TransitionCell({
    required this.node,
    required this.edge,
    required this.transition,
    required this.enabled,
    required this.onPressed,
  });

  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionEdgeV1 edge;
  final AuthoringRevision3QuestTransitionV1? transition;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(4),
    child: TextButton(
      key: Key(
        'revision3-quest-transitions-cell-${node.stableKey}-${edge.wireName}',
      ),
      onPressed: enabled ? onPressed : null,
      child: Text(_transitionStatus(transition), textAlign: TextAlign.center),
    ),
  );
}

final class _TransitionEditResult {
  const _TransitionEditResult.transition(this.transition) : remove = false;
  const _TransitionEditResult.remove() : transition = null, remove = true;

  final AuthoringRevision3QuestTransitionV1? transition;
  final bool remove;
}

class _TransitionEditorDialog extends StatefulWidget {
  const _TransitionEditorDialog({
    required this.checkpoint,
    required this.plan,
    required this.node,
    required this.edge,
    required this.current,
  });

  final Revision3QuestTransitionsEditCheckpoint checkpoint;
  final AuthoringRevision3QuestTransitionPlanV1 plan;
  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionEdgeV1 edge;
  final AuthoringRevision3QuestTransitionV1? current;

  @override
  State<_TransitionEditorDialog> createState() =>
      _TransitionEditorDialogState();
}

class _TransitionEditorDialogState extends State<_TransitionEditorDialog> {
  late bool _externalAllowed;
  late List<List<AuthoringRevision3QuestTransitionConditionAtomV1>> _groups;
  late List<AuthoringRevision3QuestTransitionEffectV1> _effects;
  late bool _succeedsParent;
  String? _error;

  bool get _optional =>
      widget.edge == AuthoringRevision3QuestTransitionEdgeV1.success ||
      widget.edge == AuthoringRevision3QuestTransitionEdgeV1.failure;

  List<AuthoringRevision3QuestTransitionNodeV1> get _nodes =>
      <AuthoringRevision3QuestTransitionNodeV1>[
        const AuthoringRevision3QuestTransitionNodeV1.root(),
        for (final slot in widget.plan.objectiveSlots)
          AuthoringRevision3QuestTransitionNodeV1.objective(slot),
      ];

  @override
  void initState() {
    super.initState();
    final current = widget.current;
    _externalAllowed = current?.externalAllowed ?? true;
    _groups = <List<AuthoringRevision3QuestTransitionConditionAtomV1>>[
      for (final group in current?.predicate?.anyOf ?? const [])
        group.allOf.toList(),
    ];
    _effects =
        current?.effects.toList() ??
        <AuthoringRevision3QuestTransitionEffectV1>[];
    _succeedsParent = current?.succeedsParent ?? false;
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const Key('revision3-quest-transition-editor'),
    title: Text(
      '${_edgeLabel(widget.edge)}: ${_nodeLabel(widget.checkpoint, widget.node)}',
    ),
    content: SizedBox(
      width: 720,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SwitchListTile(
              key: const Key('revision3-quest-transition-external'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Allow the game to trigger this directly'),
              subtitle: const Text(
                'This remains independent from the optional conditions below. Both may be enabled.',
              ),
              value: _externalAllowed,
              onChanged: (value) => setState(() {
                _externalAllowed = value;
                _error = null;
              }),
            ),
            const Divider(),
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Optional conditions',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                TextButton.icon(
                  key: const Key('revision3-quest-transition-add-alternative'),
                  onPressed: _groups.length < 8 ? _addAlternative : null,
                  icon: const Icon(Icons.add),
                  label: const Text('Add alternative'),
                ),
              ],
            ),
            const Text(
              'Any alternative may trigger this step. Inside one alternative, every condition must match.',
            ),
            if (_groups.isEmpty)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 10),
                child: Text('No automatic condition.'),
              ),
            for (var groupIndex = 0; groupIndex < _groups.length; groupIndex++)
              _conditionGroup(groupIndex),
            if (widget.edge !=
                AuthoringRevision3QuestTransitionEdgeV1.availability) ...[
              const Divider(),
              Row(
                children: [
                  Expanded(
                    child: Text(
                      'Follow-up actions',
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                  ),
                  TextButton.icon(
                    key: const Key('revision3-quest-transition-add-effect'),
                    onPressed: _effects.length < 8 ? _addEffect : null,
                    icon: const Icon(Icons.add),
                    label: const Text('Add action'),
                  ),
                ],
              ),
              const Text(
                'Start, succeed, or fail another Quest part when this step happens.',
              ),
              if (_effects.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 10),
                  child: Text('No follow-up action.'),
                ),
              for (var index = 0; index < _effects.length; index++)
                _effectRow(index),
            ],
            if (widget.node.kind ==
                    AuthoringRevision3QuestTransitionNodeKind.objective &&
                widget.edge ==
                    AuthoringRevision3QuestTransitionEdgeV1.success) ...[
              const Divider(),
              CheckboxListTile(
                key: const Key('revision3-quest-transition-completes-quest'),
                contentPadding: EdgeInsets.zero,
                title: const Text('Also complete the Quest'),
                value: _succeedsParent,
                onChanged: (value) =>
                    setState(() => _succeedsParent = value ?? false),
              ),
            ],
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(
                _error!,
                key: const Key('revision3-quest-transition-editor-error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
    ),
    actions: [
      if (_optional && widget.current != null)
        TextButton(
          key: const Key('revision3-quest-transition-remove'),
          onPressed: () =>
              Navigator.of(context).pop(const _TransitionEditResult.remove()),
          child: const Text('Remove behavior'),
        ),
      TextButton(
        key: const Key('revision3-quest-transition-cancel'),
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('revision3-quest-transition-apply'),
        onPressed: _apply,
        child: const Text('Apply'),
      ),
    ],
  );

  Widget _conditionGroup(int groupIndex) {
    final atoms = _groups[groupIndex];
    return Card.outlined(
      key: ValueKey('revision3-quest-transition-group-$groupIndex'),
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(child: Text('Alternative ${groupIndex + 1}')),
                IconButton(
                  key: ValueKey(
                    'revision3-quest-transition-remove-group-$groupIndex',
                  ),
                  tooltip: 'Remove alternative',
                  onPressed: () => setState(() {
                    _groups.removeAt(groupIndex);
                    _error = null;
                  }),
                  icon: const Icon(Icons.delete_outline),
                ),
              ],
            ),
            for (var atomIndex = 0; atomIndex < atoms.length; atomIndex++)
              _conditionRow(groupIndex, atomIndex),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                key: ValueKey(
                  'revision3-quest-transition-add-condition-$groupIndex',
                ),
                onPressed: atoms.length < 8
                    ? () => _addCondition(groupIndex)
                    : null,
                icon: const Icon(Icons.add),
                label: const Text('Add required condition'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _conditionRow(int groupIndex, int atomIndex) {
    final atom = _groups[groupIndex][atomIndex];
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        children: [
          Expanded(
            flex: 3,
            child:
                DropdownButtonFormField<
                  AuthoringRevision3QuestTransitionNodeV1
                >(
                  initialValue: atom.node,
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: 'Quest part'),
                  items: [
                    for (final node in _nodes)
                      DropdownMenuItem(
                        value: node,
                        child: Text(
                          _nodeLabel(widget.checkpoint, node),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                  ],
                  onChanged: (node) {
                    if (node == null) return;
                    _replaceAtom(
                      groupIndex,
                      atomIndex,
                      AuthoringRevision3QuestTransitionConditionAtomV1(
                        node: node,
                        test: atom.test,
                        negated: atom.negated,
                      ),
                    );
                  },
                ),
          ),
          const SizedBox(width: 8),
          Expanded(
            flex: 2,
            child:
                DropdownButtonFormField<
                  AuthoringRevision3QuestTransitionStateTestV1
                >(
                  initialValue: atom.test,
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: 'State'),
                  items: [
                    for (final state
                        in AuthoringRevision3QuestTransitionStateTestV1.values)
                      DropdownMenuItem(
                        value: state,
                        child: Text(_stateLabel(state)),
                      ),
                  ],
                  onChanged: (state) {
                    if (state == null) return;
                    _replaceAtom(
                      groupIndex,
                      atomIndex,
                      AuthoringRevision3QuestTransitionConditionAtomV1(
                        node: atom.node,
                        test: state,
                        negated: atom.negated,
                      ),
                    );
                  },
                ),
          ),
          const SizedBox(width: 4),
          Tooltip(
            message: 'Condition must not match',
            child: Checkbox(
              value: atom.negated,
              onChanged: (negated) => _replaceAtom(
                groupIndex,
                atomIndex,
                AuthoringRevision3QuestTransitionConditionAtomV1(
                  node: atom.node,
                  test: atom.test,
                  negated: negated ?? false,
                ),
              ),
            ),
          ),
          IconButton(
            tooltip: 'Remove condition',
            onPressed: () => setState(() {
              _groups[groupIndex].removeAt(atomIndex);
              if (_groups[groupIndex].isEmpty) _groups.removeAt(groupIndex);
              _error = null;
            }),
            icon: const Icon(Icons.close),
          ),
        ],
      ),
    );
  }

  Widget _effectRow(int index) {
    final effect = _effects[index];
    final targets = _nodes.where((node) => node != widget.node).toList();
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          Expanded(
            child:
                DropdownButtonFormField<
                  AuthoringRevision3QuestTransitionEffectKindV1
                >(
                  initialValue: effect.effect,
                  decoration: const InputDecoration(labelText: 'Action'),
                  items: [
                    for (final kind
                        in AuthoringRevision3QuestTransitionEffectKindV1.values)
                      DropdownMenuItem(
                        value: kind,
                        child: Text(_effectLabel(kind)),
                      ),
                  ],
                  onChanged: (kind) {
                    if (kind == null) return;
                    setState(() {
                      _effects[index] =
                          AuthoringRevision3QuestTransitionEffectV1(
                            target: effect.target,
                            effect: kind,
                          );
                      _error = null;
                    });
                  },
                ),
          ),
          const SizedBox(width: 8),
          Expanded(
            flex: 2,
            child:
                DropdownButtonFormField<
                  AuthoringRevision3QuestTransitionNodeV1
                >(
                  initialValue: effect.target,
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: 'Quest part'),
                  items: [
                    for (final node in targets)
                      DropdownMenuItem(
                        value: node,
                        child: Text(
                          _nodeLabel(widget.checkpoint, node),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                  ],
                  onChanged: (target) {
                    if (target == null) return;
                    setState(() {
                      _effects[index] =
                          AuthoringRevision3QuestTransitionEffectV1(
                            target: target,
                            effect: effect.effect,
                          );
                      _error = null;
                    });
                  },
                ),
          ),
          IconButton(
            tooltip: 'Remove action',
            onPressed: () => setState(() {
              _effects.removeAt(index);
              _error = null;
            }),
            icon: const Icon(Icons.close),
          ),
        ],
      ),
    );
  }

  void _addAlternative() {
    setState(() {
      _groups.add(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        _defaultAtom(),
      ]);
      _error = null;
    });
  }

  void _addCondition(int groupIndex) {
    setState(() {
      _groups[groupIndex].add(_defaultAtom());
      _error = null;
    });
  }

  AuthoringRevision3QuestTransitionConditionAtomV1 _defaultAtom() =>
      const AuthoringRevision3QuestTransitionConditionAtomV1(
        node: AuthoringRevision3QuestTransitionNodeV1.root(),
        test: AuthoringRevision3QuestTransitionStateTestV1.started,
        negated: false,
      );

  void _replaceAtom(
    int groupIndex,
    int atomIndex,
    AuthoringRevision3QuestTransitionConditionAtomV1 atom,
  ) {
    setState(() {
      _groups[groupIndex][atomIndex] = atom;
      _error = null;
    });
  }

  void _addEffect() {
    final target = _nodes.firstWhere((node) => node != widget.node);
    setState(() {
      _effects.add(
        AuthoringRevision3QuestTransitionEffectV1(
          target: target,
          effect: AuthoringRevision3QuestTransitionEffectKindV1.start,
        ),
      );
      _error = null;
    });
  }

  void _apply() {
    try {
      final predicate = _groups.isEmpty
          ? null
          : Revision3QuestTransitionsAuthoringService.predicate(_groups);
      final transition = AuthoringRevision3QuestTransitionV1(
        node: widget.node,
        edge: widget.edge,
        externalAllowed: _externalAllowed,
        predicate: predicate,
        effects: Revision3QuestTransitionsAuthoringService.canonicalEffects(
          _effects,
        ),
        succeedsParent: _succeedsParent,
      );
      Navigator.of(context).pop(_TransitionEditResult.transition(transition));
    } on FormatException catch (error) {
      setState(() => _error = error.message);
    }
  }
}

AuthoringRevision3QuestTransitionV1? _transitionFor(
  AuthoringRevision3QuestTransitionPlanV1 plan,
  AuthoringRevision3QuestTransitionNodeV1 node,
  AuthoringRevision3QuestTransitionEdgeV1 edge,
) {
  final key = '${node.stableKey}:${edge.wireName}';
  for (final transition in plan.transitions) {
    if (transition.stableKey == key) return transition;
  }
  return null;
}

String _nodeLabel(
  Revision3QuestTransitionsEditCheckpoint checkpoint,
  AuthoringRevision3QuestTransitionNodeV1 node,
) => switch (node.kind) {
  AuthoringRevision3QuestTransitionNodeKind.root => 'Main Quest',
  AuthoringRevision3QuestTransitionNodeKind.objective =>
    checkpoint.objectiveTitle(node.slot!),
};

String _transitionStatus(AuthoringRevision3QuestTransitionV1? transition) {
  if (transition == null) return 'Not used';
  if (transition.externalAllowed &&
      transition.predicate == null &&
      transition.effects.isEmpty &&
      !transition.succeedsParent) {
    return 'Engine default';
  }
  return 'Configured';
}

String _edgeLabel(AuthoringRevision3QuestTransitionEdgeV1 edge) =>
    switch (edge) {
      AuthoringRevision3QuestTransitionEdgeV1.availability => 'Available',
      AuthoringRevision3QuestTransitionEdgeV1.start => 'Start',
      AuthoringRevision3QuestTransitionEdgeV1.success => 'Success',
      AuthoringRevision3QuestTransitionEdgeV1.failure => 'Failure',
    };

String _stateLabel(AuthoringRevision3QuestTransitionStateTestV1 state) =>
    switch (state) {
      AuthoringRevision3QuestTransitionStateTestV1.available => 'Available',
      AuthoringRevision3QuestTransitionStateTestV1.running => 'Running',
      AuthoringRevision3QuestTransitionStateTestV1.started => 'Started',
      AuthoringRevision3QuestTransitionStateTestV1.succeeded => 'Succeeded',
      AuthoringRevision3QuestTransitionStateTestV1.failed => 'Failed',
      AuthoringRevision3QuestTransitionStateTestV1.completed => 'Completed',
    };

String _effectLabel(AuthoringRevision3QuestTransitionEffectKindV1 effect) =>
    switch (effect) {
      AuthoringRevision3QuestTransitionEffectKindV1.start => 'Start',
      AuthoringRevision3QuestTransitionEffectKindV1.succeed => 'Succeed',
      AuthoringRevision3QuestTransitionEffectKindV1.fail => 'Fail',
    };

String _transitionErrorMessage(String code) {
  if (code.contains('NO_CHANGES')) {
    return 'Change at least one Quest behavior before saving.';
  }
  if (code.contains('HEAD') ||
      code.contains('PROJECT_CONFLICT') ||
      code.contains('QUEST_CONFLICT') ||
      code.contains('TARGET_CONFLICT')) {
    return 'The project changed while this editor was open. Close it and reopen the Quest from the refreshed library.';
  }
  if (code.contains('TRANSITION_PLAN') || code.contains('REQUEST')) {
    return 'Review the highlighted Quest behavior and try again.';
  }
  return 'Quest behavior could not be saved safely. Nothing was built, deployed, or written into the game.';
}
