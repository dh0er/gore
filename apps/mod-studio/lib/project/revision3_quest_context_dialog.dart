import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_quest_context_authoring.dart';

/// Separate atomic editor for description and Story-catalog connections.
/// Outline editing deliberately remains a no-game-root operation.
class Revision3QuestContextEditDialog extends StatefulWidget {
  const Revision3QuestContextEditDialog({
    required this.index,
    required this.quest,
    required this.gameRoot,
    required this.service,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final String gameRoot;
  final Revision3QuestContextAuthoringService service;

  @override
  State<Revision3QuestContextEditDialog> createState() =>
      _Revision3QuestContextEditDialogState();
}

class _Revision3QuestContextEditDialogState
    extends State<Revision3QuestContextEditDialog> {
  final _description = TextEditingController();
  Revision3QuestContextEditCheckpoint? _checkpoint;
  String? _parentCatalogId;
  String? _giverCatalogId;
  String? _error;
  bool _loading = true;
  bool _busy = false;
  bool _checkpointLocked = false;
  bool _reviewRequired = false;
  bool _allowPop = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _description.addListener(_fieldChanged);
    _load();
  }

  @override
  void dispose() {
    _loadGeneration++;
    _description.removeListener(_fieldChanged);
    _description.dispose();
    super.dispose();
  }

  void _fieldChanged() {
    if (mounted && !_reviewRequired) setState(() => _error = null);
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
        gameRoot: widget.gameRoot,
      );
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _checkpoint = checkpoint;
        _description.text = checkpoint.seed.description;
        _parentCatalogId = checkpoint.currentParent.catalogId;
        _giverCatalogId = checkpoint.currentGiver.catalogId;
        _reviewRequired = false;
        _loading = false;
      });
    } on Revision3QuestContextStaleCheckpointException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _checkpointLocked = true;
        _error =
            'The project changed while this editor opened. Close it and reopen the Quest from the refreshed library.';
      });
    } on Revision3QuestContextRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _checkpointLocked = true;
        _error =
            'The project checkpoint can no longer be verified. Close this editor and reopen the managed project.';
      });
    } on Revision3QuestContextUnavailableException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _checkpointLocked = true;
        _error =
            'The Quest\'s current family or giver is unavailable in this game catalog. This editor cannot guess a replacement.';
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _error =
            'Quest details and game choices could not be loaded. No project or game files were changed.';
      });
    }
  }

  bool get _hasChanges {
    final checkpoint = _checkpoint;
    if (checkpoint == null) return false;
    return _reviewRequired ||
        _description.text != checkpoint.seed.description ||
        _parentCatalogId != checkpoint.currentParent.catalogId ||
        _giverCatalogId != checkpoint.currentGiver.catalogId;
  }

  bool get _canSave =>
      !_reviewRequired &&
      _parentCatalogId != null &&
      _giverCatalogId != null &&
      _hasChanges;

  Future<void> _save() async {
    final checkpoint = _checkpoint;
    final parentId = _parentCatalogId;
    final giverId = _giverCatalogId;
    if (_busy ||
        _checkpointLocked ||
        checkpoint == null ||
        parentId == null ||
        giverId == null) {
      return;
    }
    final problem = Revision3QuestContextAuthoringService.validateDescription(
      _description.text,
    );
    if (problem != null) {
      setState(() => _error = problem);
      return;
    }
    final parent = checkpoint.catalog.parent(parentId);
    final giver = checkpoint.catalog.giver(giverId);
    if (parent == null || giver == null) {
      setState(() {
        _error = 'Review the Quest family and giver before saving.';
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.service.publish(
        checkpoint: checkpoint,
        gameRoot: widget.gameRoot,
        description: _description.text,
        parent: parent,
        giver: giver,
      );
      if (!mounted) return;
      await _popAfterUnlock(publication);
    } on Revision3QuestContextCatalogDriftException catch (drift) {
      if (!mounted) return;
      try {
        final reviewed = checkpoint.withCatalogForReview(drift.freshCatalog);
        setState(() {
          _checkpoint = reviewed;
          _parentCatalogId = null;
          _giverCatalogId = null;
          _reviewRequired = true;
          _busy = false;
          _error =
              'The game choices changed while this editor was open. Review the Quest family and giver again before saving.';
        });
      } on Revision3QuestContextUnavailableException {
        setState(() {
          _busy = false;
          _checkpointLocked = true;
          _error =
              'The Quest\'s current family or giver disappeared from the refreshed game catalog. Close this editor; no replacement was guessed.';
        });
      }
    } on Revision3QuestContextUnavailableException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The Quest\'s current family or giver disappeared from the refreshed game catalog. Close this editor; no replacement was guessed.';
      });
    } on Revision3QuestContextStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project changed while this editor was open. Close it and reopen the Quest from the refreshed library.';
      });
    } on Revision3QuestContextRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _checkpointLocked = true;
        _error =
            'The project checkpoint can no longer be verified. Close this editor and reopen the managed project.';
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = _contextErrorMessage(error.code);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'The Quest details could not be saved. Nothing was built, deployed, or written into the game.';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final checkpoint = _checkpoint;
    final enabled = !_loading && !_busy && !_checkpointLocked;
    return PopScope(
      canPop: _allowPop || (!_busy && !_hasChanges),
      child: AlertDialog(
        key: const Key('revision3-quest-context-dialog'),
        title: const Text('Edit Quest details'),
        content: SizedBox(
          width: 660,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  widget.quest.displayName,
                  key: const Key('revision3-quest-context-quest-name'),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                const Text(
                  'Edit the player-facing description and connect this Quest to a game Quest family and giver.',
                ),
                const SizedBox(height: 12),
                Container(
                  key: const Key('revision3-quest-context-boundary'),
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Text(
                    'This saves an offline project draft only. Build remains blocked, runtime behavior remains unqualified, and nothing is deployed to the game.',
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
                else if (checkpoint != null) ...[
                  TextField(
                    key: const Key('revision3-quest-context-description'),
                    controller: _description,
                    enabled: enabled,
                    minLines: 2,
                    maxLines: 4,
                    maxLength: 512,
                    textInputAction: TextInputAction.done,
                    inputFormatters: [
                      FilteringTextInputFormatter.allow(
                        RegExp(r'[\x20-\x21\x23-\x5b\x5d-\x7e]'),
                      ),
                      LengthLimitingTextInputFormatter(512),
                    ],
                    decoration: const InputDecoration(
                      labelText: 'Quest description',
                      helperText:
                          'One line of basic Latin text, up to 512 characters; quotes and backslashes are not supported.',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 16),
                  DropdownButtonFormField<String>(
                    key: ValueKey(
                      'revision3-quest-context-parent-${checkpoint.catalog.catalogSeal?.sha256}',
                    ),
                    initialValue: _parentCatalogId,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      labelText: 'Quest family',
                      helperText:
                          'The existing game Quest this draft belongs to.',
                      border: OutlineInputBorder(),
                    ),
                    items: [
                      for (final choice in checkpoint.catalog.parents)
                        DropdownMenuItem(
                          value: choice.catalogId,
                          child: Text(
                            choice.displayLabel,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                    ],
                    onChanged: enabled
                        ? (value) => setState(() {
                            _parentCatalogId = value;
                            if (_giverCatalogId != null) {
                              _reviewRequired = false;
                              _error = null;
                            }
                          })
                        : null,
                  ),
                  const SizedBox(height: 14),
                  DropdownButtonFormField<String>(
                    key: ValueKey(
                      'revision3-quest-context-giver-${checkpoint.catalog.catalogSeal?.sha256}',
                    ),
                    initialValue: _giverCatalogId,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      labelText: 'Quest giver',
                      helperText: 'The character who introduces this Quest.',
                      border: OutlineInputBorder(),
                    ),
                    items: [
                      for (final choice in checkpoint.catalog.givers)
                        DropdownMenuItem(
                          value: choice.catalogId,
                          child: Text(
                            choice.displayLabel,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                    ],
                    onChanged: enabled
                        ? (value) => setState(() {
                            _giverCatalogId = value;
                            if (_parentCatalogId != null) {
                              _reviewRequired = false;
                              _error = null;
                            }
                          })
                        : null,
                  ),
                ],
                if (_error != null) ...[
                  const SizedBox(height: 14),
                  Semantics(
                    liveRegion: true,
                    child: Text(
                      _error!,
                      key: const Key('revision3-quest-context-error'),
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
              key: const Key('revision3-quest-context-retry'),
              onPressed: _busy ? null : _load,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          TextButton(
            key: const Key('revision3-quest-context-cancel'),
            onPressed: _busy ? null : _cancel,
            child: Text(_checkpointLocked ? 'Close' : 'Cancel'),
          ),
          FilledButton.icon(
            key: const Key('revision3-quest-context-save'),
            onPressed: enabled && _canSave ? _save : null,
            icon: _busy
                ? const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.save_outlined),
            label: Text(_busy ? 'Saving…' : 'Save details'),
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
        key: const Key('revision3-quest-context-discard-dialog'),
        title: const Text('Discard Quest changes?'),
        content: const Text(
          'Your unsaved description and connections will be lost.',
        ),
        actions: [
          TextButton(
            key: const Key('revision3-quest-context-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Keep editing'),
          ),
          FilledButton(
            key: const Key('revision3-quest-context-discard'),
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
    Revision3QuestContextEditPublication? result,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(result);
  }
}

String _contextErrorMessage(String code) {
  if (code.contains('CATALOG')) {
    return 'The game choices changed or are unavailable. Close this editor, verify the configured game installation, and try again.';
  }
  if (code.contains('NO_CHANGES')) {
    return 'Change at least one Quest detail before saving.';
  }
  if (code.contains('REQUEST')) {
    return 'Review the description, Quest family, and giver before saving.';
  }
  if (code.contains('INPUT') ||
      code.contains('PRISTINE') ||
      code.contains('UNSUPPORTED_GENERATION') ||
      code.contains('TARGET') ||
      code.contains('RECOVERY') ||
      code.contains('STORE_GAME_ALIAS')) {
    return 'The configured game installation changed, is incomplete, or cannot be read safely. Verify it in Settings, then close and reopen this editor before retrying.';
  }
  return 'The Quest details could not be saved safely. Nothing was built, deployed, or written into the game.';
}
