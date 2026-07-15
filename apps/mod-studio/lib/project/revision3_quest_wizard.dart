import 'dart:convert';

import 'package:flutter/material.dart';

import 'revision3_quest_authoring.dart';

/// First non-technical authoring surface over the exact managed-R3 Quest
/// transaction. It deliberately exposes no generated identity, namespace,
/// source path, build, deployment, game-write, or runtime control.
class Revision3QuestWizardDialog extends StatefulWidget {
  const Revision3QuestWizardDialog({
    required this.gameRoot,
    required this.loadCatalog,
    required this.publish,
    this.initialParentCatalogId,
    this.initialGiverCatalogId,
    super.key,
  });

  final String gameRoot;
  final Revision3QuestCatalogLoader loadCatalog;
  final Revision3QuestDraftPublisher publish;
  final String? initialParentCatalogId;
  final String? initialGiverCatalogId;

  @override
  State<Revision3QuestWizardDialog> createState() =>
      _Revision3QuestWizardDialogState();
}

class _Revision3QuestWizardDialogState
    extends State<Revision3QuestWizardDialog> {
  final _formKey = GlobalKey<FormState>();
  final _title = TextEditingController();
  final _description = TextEditingController();
  final List<TextEditingController> _objectives = <TextEditingController>[
    TextEditingController(),
  ];

  Revision3QuestCatalog? _catalog;
  String? _parentCatalogId;
  String? _giverCatalogId;
  String? _explicitParentCatalogId;
  String? _explicitGiverCatalogId;
  String? _catalogSelectionWarning;
  String? _error;
  bool _catalogLoading = true;
  bool _publishing = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  int _loadGeneration = 0;
  int _catalogEpoch = 0;

  @override
  void initState() {
    super.initState();
    _explicitParentCatalogId = widget.initialParentCatalogId;
    _explicitGiverCatalogId = widget.initialGiverCatalogId;
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3QuestWizardDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.gameRoot != widget.gameRoot) _loadCatalog(clear: true);
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    _title.dispose();
    _description.dispose();
    for (final objective in _objectives) {
      objective.dispose();
    }
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _catalogLoading = true;
      _error = null;
      if (clear) _catalog = null;
    });
    try {
      final catalog = await widget.loadCatalog(widget.gameRoot);
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _adoptCatalog(catalog, chooseDefaults: true);
        _catalogLoading = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalogLoading = false;
        _error =
            'Quest families and givers could not be refreshed from the configured game. No project or game files were changed.';
      });
    }
  }

  void _adoptCatalog(
    Revision3QuestCatalog catalog, {
    required bool chooseDefaults,
  }) {
    final preferredParent = _explicitParentCatalogId ?? _parentCatalogId;
    final preferredGiver = _explicitGiverCatalogId ?? _giverCatalogId;
    final explicitParentMissing =
        _explicitParentCatalogId != null &&
        !catalog.containsParent(_explicitParentCatalogId!);
    final explicitGiverMissing =
        _explicitGiverCatalogId != null &&
        !catalog.containsGiver(_explicitGiverCatalogId!);
    _catalog = catalog;
    _catalogEpoch += 1;
    _parentCatalogId = catalog.containsParent(preferredParent ?? '')
        ? preferredParent
        : chooseDefaults && _explicitParentCatalogId == null
        ? catalog.parents.first.catalogId
        : null;
    _giverCatalogId = catalog.containsGiver(preferredGiver ?? '')
        ? preferredGiver
        : chooseDefaults && _explicitGiverCatalogId == null
        ? catalog.givers.first.catalogId
        : null;
    _catalogSelectionWarning = _missingCatalogChoiceMessage(
      parentMissing: explicitParentMissing,
      giverMissing: explicitGiverMissing,
    );
  }

  void _selectParent(String? value) {
    setState(() {
      _parentCatalogId = value;
      if (value != null) _explicitParentCatalogId = value;
      _clearResolvedCatalogSelectionWarning();
    });
  }

  void _selectGiver(String? value) {
    setState(() {
      _giverCatalogId = value;
      if (value != null) _explicitGiverCatalogId = value;
      _clearResolvedCatalogSelectionWarning();
    });
  }

  void _clearResolvedCatalogSelectionWarning() {
    if (_parentCatalogId != null && _giverCatalogId != null) {
      _catalogSelectionWarning = null;
    }
  }

  Future<void> _submit() async {
    if (_publishing ||
        _requiresReopen ||
        _staleCheckpoint ||
        _catalog == null) {
      return;
    }
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final parentCatalogId = _parentCatalogId;
    final giverCatalogId = _giverCatalogId;
    if (parentCatalogId == null || giverCatalogId == null) {
      setState(() => _error = 'Choose a Quest family and a Quest giver.');
      return;
    }

    final input = Revision3QuestDraftAuthoringInput(
      parentCatalogId: parentCatalogId,
      giverCatalogId: giverCatalogId,
      title: _title.text,
      description: _description.text,
      objectiveTitle: _objectives.first.text,
      additionalObjectiveTitles: _objectives
          .skip(1)
          .map((controller) => controller.text)
          .toList(growable: false),
    );
    setState(() {
      _publishing = true;
      _publicationStarted = false;
      _error = null;
    });

    var completed = false;
    try {
      // Rebuild immediately before publication. The native transaction also
      // resolves these selector IDs from its own fresh trusted catalog.
      final fresh = await widget.loadCatalog(widget.gameRoot);
      if (!mounted) return;
      if (!fresh.containsParent(parentCatalogId) ||
          !fresh.containsGiver(giverCatalogId)) {
        setState(() {
          _adoptCatalog(fresh, chooseDefaults: false);
          _publishing = false;
          _publicationStarted = false;
          _error =
              'The game choices changed while this wizard was open. Review the highlighted choices and try again.';
        });
        return;
      }

      setState(() => _publicationStarted = true);
      final publication = await widget.publish(
        gameRoot: widget.gameRoot,
        input: input,
      );
      if (!mounted) return;
      completed = true;
      Navigator.of(context).pop(publication);
    } on Revision3QuestDraftRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _publishing = false;
        _publicationStarted = false;
        _error =
            'This project can no longer be verified as current. Close this wizard and reopen the managed project before continuing.';
      });
    } on Revision3QuestDraftStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _publishing = false;
        _publicationStarted = false;
        _error =
            'The managed project changed while this wizard was open. Close this wizard and create the Quest draft again from the current project.';
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _publicationStarted = false;
        _error =
            'The Quest draft could not be saved. Nothing was compiled, deployed, or written into the game. You can review the form and try again.';
      });
    } finally {
      if (mounted && !completed && _publishing) {
        setState(() {
          _publishing = false;
          _publicationStarted = false;
        });
      }
    }
  }

  void _addObjective() {
    if (_objectives.length >= 8 || _publishing) return;
    setState(() => _objectives.add(TextEditingController()));
  }

  void _removeObjective(int index) {
    if (_objectives.length <= 1 || _publishing) return;
    final removed = _objectives.removeAt(index);
    removed.dispose();
    setState(() {});
  }

  void _moveObjective(int index, int delta) {
    final target = index + delta;
    if (_publishing || target < 0 || target >= _objectives.length) return;
    setState(() {
      final controller = _objectives.removeAt(index);
      _objectives.insert(target, controller);
    });
  }

  String? _validateObjective(String? value, int index) {
    final error = _validateQuestText(
      value,
      label: 'Enter objective ${index + 1}',
      maxBytes: 128,
    );
    if (error != null) return error;
    final normalized = value!.trim().toLowerCase();
    for (var other = 0; other < _objectives.length; other++) {
      if (other != index &&
          _objectives[other].text.trim().toLowerCase() == normalized) {
        return 'Each objective must be different';
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final busy = _catalogLoading || _publishing;
    return PopScope(
      canPop: !_publicationStarted,
      child: AlertDialog(
        key: const Key('revision3-quest-wizard'),
        title: const Row(
          children: [
            Icon(Icons.assignment_add),
            SizedBox(width: 10),
            Expanded(child: Text('Create a Quest draft')),
          ],
        ),
        content: SizedBox(
          width: 680,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 650),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const _DraftBoundaryBanner(),
                  const SizedBox(height: 16),
                  if (_catalogSelectionWarning != null) ...[
                    _WizardMessage(
                      key: const Key(
                        'revision3-quest-catalog-selection-warning',
                      ),
                      message: _catalogSelectionWarning!,
                      error: false,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_error != null) ...[
                    _WizardMessage(
                      key: const Key('revision3-quest-wizard-error'),
                      message: _error!,
                      error: true,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_catalogLoading)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 40),
                      child: Center(
                        child: Semantics(
                          liveRegion: true,
                          label: 'Refreshing Quest choices from the game',
                          child: const CircularProgressIndicator(
                            key: Key('revision3-quest-catalog-loading'),
                          ),
                        ),
                      ),
                    )
                  else if (_catalog == null)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-quest-catalog-retry'),
                        onPressed: _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: const Text('Refresh game choices'),
                      ),
                    )
                  else
                    _buildForm(context),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-quest-cancel'),
            onPressed: _publicationStarted
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(
              _requiresReopen || _staleCheckpoint ? 'Close' : 'Cancel',
            ),
          ),
          FilledButton.icon(
            key: const Key('revision3-quest-submit'),
            onPressed:
                busy || _catalog == null || _requiresReopen || _staleCheckpoint
                ? null
                : _submit,
            icon: _publishing
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.save_outlined),
            label: const Text('Save draft to project'),
          ),
        ],
      ),
    );
  }

  Widget _buildForm(BuildContext context) {
    final catalog = _catalog!;
    final enabled = !_publishing && !_requiresReopen && !_staleCheckpoint;
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Story basics', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
            'This first draft format currently supports plain Latin text without accented letters, line breaks, quotation marks, or backslashes.',
          ),
          const SizedBox(height: 10),
          TextFormField(
            key: const Key('revision3-quest-title'),
            controller: _title,
            enabled: enabled,
            maxLength: 128,
            textInputAction: TextInputAction.next,
            decoration: const InputDecoration(
              labelText: 'Quest name',
              hintText: 'Find the missing scout',
              helperText: 'This is the name shown to players.',
              border: OutlineInputBorder(),
            ),
            validator: (value) => _validateQuestText(
              value,
              label: 'Enter a Quest name',
              maxBytes: 128,
            ),
          ),
          const SizedBox(height: 10),
          TextFormField(
            key: const Key('revision3-quest-description'),
            controller: _description,
            enabled: enabled,
            maxLength: 512,
            maxLines: 3,
            decoration: const InputDecoration(
              labelText: 'What is this Quest about?',
              hintText: 'Someone vanished near the old gate.',
              border: OutlineInputBorder(),
            ),
            validator: (value) => _validateQuestText(
              value,
              label: 'Describe the Quest',
              maxBytes: 512,
            ),
          ),
          const SizedBox(height: 10),
          Text('Objectives', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
            'Add up to eight steps. Their order is saved in the project; runtime transitions are not yet qualified.',
          ),
          const SizedBox(height: 10),
          for (var index = 0; index < _objectives.length; index++) ...[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: TextFormField(
                    key: Key(
                      index == 0
                          ? 'revision3-quest-objective'
                          : 'revision3-quest-objective-$index',
                    ),
                    controller: _objectives[index],
                    enabled: enabled,
                    maxLength: 128,
                    textInputAction: TextInputAction.next,
                    decoration: InputDecoration(
                      labelText: 'Objective ${index + 1}',
                      hintText: index == 0
                          ? 'Ask the guards what happened'
                          : 'Describe the next step',
                      border: const OutlineInputBorder(),
                    ),
                    validator: (value) => _validateObjective(value, index),
                  ),
                ),
                const SizedBox(width: 6),
                Column(
                  children: [
                    IconButton(
                      key: Key('revision3-quest-objective-up-$index'),
                      tooltip: 'Move objective up',
                      onPressed: enabled && index > 0
                          ? () => _moveObjective(index, -1)
                          : null,
                      icon: const Icon(Icons.arrow_upward),
                    ),
                    IconButton(
                      key: Key('revision3-quest-objective-down-$index'),
                      tooltip: 'Move objective down',
                      onPressed: enabled && index + 1 < _objectives.length
                          ? () => _moveObjective(index, 1)
                          : null,
                      icon: const Icon(Icons.arrow_downward),
                    ),
                    IconButton(
                      key: Key('revision3-quest-objective-remove-$index'),
                      tooltip: 'Remove objective',
                      onPressed: enabled && _objectives.length > 1
                          ? () => _removeObjective(index)
                          : null,
                      icon: const Icon(Icons.remove_circle_outline),
                    ),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 6),
          ],
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              key: const Key('revision3-quest-objective-add'),
              onPressed: enabled && _objectives.length < 8
                  ? _addObjective
                  : null,
              icon: const Icon(Icons.add),
              label: const Text('Add objective'),
            ),
          ),
          const SizedBox(height: 18),
          Text(
            'Connect it to the world',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 10),
          DropdownButtonFormField<String>(
            key: ValueKey('revision3-quest-parent-$_catalogEpoch'),
            initialValue: _parentCatalogId,
            decoration: const InputDecoration(
              labelText: 'Quest family',
              helperText:
                  'Choose the existing game Quest this draft belongs to.',
              border: OutlineInputBorder(),
            ),
            isExpanded: true,
            items: [
              for (final choice in catalog.parents)
                DropdownMenuItem(
                  value: choice.catalogId,
                  child: Text(
                    choice.displayName,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
            ],
            onChanged: enabled ? _selectParent : null,
            validator: (value) =>
                value == null ? 'Choose a Quest family' : null,
          ),
          const SizedBox(height: 14),
          DropdownButtonFormField<String>(
            key: ValueKey('revision3-quest-giver-$_catalogEpoch'),
            initialValue: _giverCatalogId,
            decoration: const InputDecoration(
              labelText: 'Quest giver',
              helperText: 'Choose the character who introduces this Quest.',
              border: OutlineInputBorder(),
            ),
            isExpanded: true,
            items: [
              for (final choice in catalog.givers)
                DropdownMenuItem(
                  value: choice.catalogId,
                  child: Text(
                    choice.displayName,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
            ],
            onChanged: enabled ? _selectGiver : null,
            validator: (value) => value == null ? 'Choose a Quest giver' : null,
          ),
          const SizedBox(height: 14),
          const Text(
            'Technical names, entity IDs, module names, and source paths are generated automatically from the exact project checkpoint.',
          ),
        ],
      ),
    );
  }
}

String? _missingCatalogChoiceMessage({
  required bool parentMissing,
  required bool giverMissing,
}) {
  if (parentMissing && giverMissing) {
    return 'The requested Quest family and Quest giver are no longer '
        'available in the current game choices. Choose both again before '
        'saving.';
  }
  if (parentMissing) {
    return 'The requested Quest family is no longer available in the current '
        'game choices. Choose a Quest family before saving.';
  }
  if (giverMissing) {
    return 'The requested Quest giver is no longer available in the current '
        'game choices. Choose a Quest giver before saving.';
  }
  return null;
}

class _DraftBoundaryBanner extends StatelessWidget {
  const _DraftBoundaryBanner();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      label: 'Quest Draft capability limits',
      child: Container(
        key: const Key('revision3-quest-boundary'),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: scheme.secondaryContainer,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                Chip(label: Text('Offline draft')),
                Chip(label: Text('Build blocked')),
                Chip(label: Text('Runtime unqualified')),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              'This saves a Quest shell and its ordered objectives into the managed project. It does not compile, deploy, write game files, change a save, or prove that the Quest works at runtime.',
              style: TextStyle(color: scheme.onSecondaryContainer),
            ),
          ],
        ),
      ),
    );
  }
}

class _WizardMessage extends StatelessWidget {
  const _WizardMessage({required this.message, required this.error, super.key});

  final String message;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final background = error ? scheme.errorContainer : scheme.primaryContainer;
    final foreground = error
        ? scheme.onErrorContainer
        : scheme.onPrimaryContainer;
    return Semantics(
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: background,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Text(message, style: TextStyle(color: foreground)),
      ),
    );
  }
}

String? _validateQuestText(
  String? value, {
  required String label,
  required int maxBytes,
}) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty) return label;
  if (utf8.encode(normalized).length > maxBytes) return '$label is too long';
  for (final rune in normalized.runes) {
    if (rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c) {
      return 'Use plain text without line breaks, quotes, or backslashes';
    }
  }
  return null;
}
