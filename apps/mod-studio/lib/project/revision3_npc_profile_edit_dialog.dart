import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_npc_profile_edit_authoring.dart';

@immutable
final class Revision3NpcProfileEditDialogCopy {
  const Revision3NpcProfileEditDialogCopy({
    required this.title,
    required this.description,
    required this.nameLabel,
    required this.nameHint,
    required this.archetypeLabel,
    required this.archetypeHelp,
    required this.boundary,
    required this.loading,
    required this.cancel,
    required this.close,
    required this.save,
    required this.saving,
    required this.retry,
    required this.loadFailed,
    required this.catalogChanged,
    required this.currentArchetypeUnavailable,
    required this.stale,
    required this.requiresReopen,
    required this.saveFailed,
    required this.nameRequired,
    required this.nameTooLong,
    required this.nameControl,
    required this.reviewSelection,
    required this.discardTitle,
    required this.discardBody,
    required this.keepEditing,
    required this.discard,
  });

  const Revision3NpcProfileEditDialogCopy.english()
    : title = 'Edit name & archetype',
      description =
          'Change the friendly character name or choose another verified structural starting point.',
      nameLabel = 'Character name',
      nameHint = 'Shown to authors in this project.',
      archetypeLabel = 'Archetype / base character',
      archetypeHelp =
          'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.',
      boundary =
          'Only the offline project draft changes. The game installation and save games remain unchanged.',
      loading = 'Loading current NPC details...',
      cancel = 'Cancel',
      close = 'Close',
      save = 'Save changes',
      saving = 'Saving...',
      retry = 'Retry',
      loadFailed =
          'NPC details and verified archetypes could not be loaded. No files were changed.',
      catalogChanged =
          'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.',
      currentArchetypeUnavailable =
          'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.',
      stale =
          'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.',
      requiresReopen =
          'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.',
      saveFailed =
          'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.',
      nameRequired = 'Enter a character name.',
      nameTooLong = 'The character name must be at most 256 UTF-8 bytes.',
      nameControl =
          'The character name contains an unsupported control character.',
      reviewSelection = 'Review and choose an archetype before saving.',
      discardTitle = 'Discard NPC changes?',
      discardBody = 'Your unsaved name and archetype choice will be lost.',
      keepEditing = 'Keep editing',
      discard = 'Discard';

  final String title;
  final String description;
  final String nameLabel;
  final String nameHint;
  final String archetypeLabel;
  final String archetypeHelp;
  final String boundary;
  final String loading;
  final String cancel;
  final String close;
  final String save;
  final String saving;
  final String retry;
  final String loadFailed;
  final String catalogChanged;
  final String currentArchetypeUnavailable;
  final String stale;
  final String requiresReopen;
  final String saveFailed;
  final String nameRequired;
  final String nameTooLong;
  final String nameControl;
  final String reviewSelection;
  final String discardTitle;
  final String discardBody;
  final String keepEditing;
  final String discard;
}

/// Direct, friendly editor for one exact-current managed-R3 NPC profile.
///
/// The dialog owns no publication authority. It delegates catalog refresh and
/// exact-head publication to [Revision3NpcProfileEditAuthoringService].
final class Revision3NpcProfileEditDialog extends StatefulWidget {
  const Revision3NpcProfileEditDialog({
    required this.index,
    required this.npc,
    required this.gameRoot,
    required this.service,
    this.copy = const Revision3NpcProfileEditDialogCopy.english(),
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity npc;
  final String gameRoot;
  final Revision3NpcProfileEditAuthoringService service;
  final Revision3NpcProfileEditDialogCopy copy;

  @override
  State<Revision3NpcProfileEditDialog> createState() =>
      _Revision3NpcProfileEditDialogState();
}

class _Revision3NpcProfileEditDialogState
    extends State<Revision3NpcProfileEditDialog> {
  final TextEditingController _name = TextEditingController();
  Revision3NpcProfileEditCheckpoint? _checkpoint;
  String? _archetypeId;
  String? _error;
  bool _loading = true;
  bool _busy = false;
  bool _locked = false;
  bool _reviewRequired = false;
  bool _allowPop = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _name.addListener(_fieldChanged);
    unawaited(_load());
  }

  @override
  void dispose() {
    _loadGeneration++;
    _name
      ..removeListener(_fieldChanged)
      ..dispose();
    super.dispose();
  }

  void _fieldChanged() {
    if (mounted && !_reviewRequired && _error != null) {
      setState(() => _error = null);
    } else if (mounted) {
      setState(() {});
    }
  }

  Future<void> _load() async {
    if (_busy) return;
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final checkpoint = await widget.service.load(
        index: widget.index,
        npc: widget.npc,
        gameRoot: widget.gameRoot,
      );
      if (!mounted || generation != _loadGeneration) return;
      _name
        ..removeListener(_fieldChanged)
        ..text = checkpoint.seed.displayName
        ..addListener(_fieldChanged);
      setState(() {
        _checkpoint = checkpoint;
        _archetypeId = checkpoint.currentArchetype.catalogId;
        _reviewRequired = false;
        _loading = false;
      });
    } on Revision3NpcProfileEditStaleCheckpointException {
      _lockLoad(generation, widget.copy.stale);
    } on Revision3NpcProfileEditRequiresReopenException {
      _lockLoad(generation, widget.copy.requiresReopen);
    } on Revision3NpcProfileEditUnavailableException {
      _lockLoad(generation, widget.copy.currentArchetypeUnavailable);
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _error = widget.copy.loadFailed;
      });
    }
  }

  void _lockLoad(int generation, String message) {
    if (!mounted || generation != _loadGeneration) return;
    setState(() {
      _loading = false;
      _locked = true;
      _error = message;
    });
  }

  bool get _hasChanges {
    final checkpoint = _checkpoint;
    if (checkpoint == null) return false;
    final selected = checkpoint.catalog.choice(_archetypeId ?? '');
    final selectedTriple = selected?.parentTriple;
    final currentTriple = checkpoint.currentArchetype.parentTriple;
    return _reviewRequired ||
        _name.text.trim() != checkpoint.seed.displayName ||
        (selectedTriple != null &&
            currentTriple != null &&
            !selectedTriple.sameBinding(currentTriple));
  }

  bool get _canSave =>
      !_loading &&
      !_busy &&
      !_locked &&
      !_reviewRequired &&
      _checkpoint != null &&
      _archetypeId != null &&
      _hasChanges;

  String? _validateName() {
    final value = _name.text;
    if (value.trim().isEmpty) return widget.copy.nameRequired;
    if (utf8.encode(value.trim()).length > 256) return widget.copy.nameTooLong;
    if (value.runes.any(
      (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
    )) {
      return widget.copy.nameControl;
    }
    return null;
  }

  Future<void> _save() async {
    final checkpoint = _checkpoint;
    final archetypeId = _archetypeId;
    if (!_canSave || checkpoint == null || archetypeId == null) return;
    final problem = _validateName();
    if (problem != null) {
      setState(() => _error = problem);
      return;
    }
    final archetype = checkpoint.catalog.choice(archetypeId);
    if (archetype == null) {
      setState(() => _error = widget.copy.reviewSelection);
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
        displayName: _name.text,
        archetype: archetype,
      );
      if (!mounted) return;
      await _popAfterUnlock(publication);
    } on Revision3NpcProfileCatalogDriftException catch (drift) {
      if (!mounted) return;
      try {
        final reviewed = checkpoint.withCatalogForReview(drift.freshCatalog);
        setState(() {
          _checkpoint = reviewed;
          _archetypeId = null;
          _reviewRequired = true;
          _busy = false;
          _error = widget.copy.catalogChanged;
        });
      } on Revision3NpcProfileEditUnavailableException {
        _lockSave(widget.copy.currentArchetypeUnavailable);
      }
    } on Revision3NpcProfileEditUnavailableException {
      _lockSave(widget.copy.currentArchetypeUnavailable);
    } on Revision3NpcProfileEditStaleCheckpointException {
      _lockSave(widget.copy.stale);
    } on Revision3NpcProfileEditRequiresReopenException {
      _lockSave(widget.copy.requiresReopen);
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
        _error = _nativeError(error.code);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = widget.copy.saveFailed;
      });
    }
  }

  void _lockSave(String message) {
    if (!mounted) return;
    setState(() {
      _busy = false;
      _locked = true;
      _error = message;
    });
  }

  String _nativeError(String code) {
    if (code.contains('NO_CHANGES')) return widget.copy.reviewSelection;
    if (code.contains('CATALOG')) return widget.copy.catalogChanged;
    if (code.contains('HEAD') ||
        code.contains('STORE') ||
        code.contains('INVARIANT')) {
      return widget.copy.requiresReopen;
    }
    return widget.copy.saveFailed;
  }

  @override
  Widget build(BuildContext context) {
    final checkpoint = _checkpoint;
    final enabled = !_loading && !_busy && !_locked;
    return PopScope<Revision3NpcProfileEditPublication?>(
      canPop: _allowPop || (!_busy && !_hasChanges),
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop && !_busy) unawaited(_cancel());
      },
      child: AlertDialog(
        key: const Key('revision3-npc-profile-edit-dialog'),
        title: Text(widget.copy.title),
        content: SizedBox(
          width: 560,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(widget.copy.description),
                const SizedBox(height: 12),
                Container(
                  key: const Key('revision3-npc-profile-edit-boundary'),
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(widget.copy.boundary),
                ),
                const SizedBox(height: 16),
                if (_loading)
                  Semantics(
                    liveRegion: true,
                    label: widget.copy.loading,
                    child: const Center(
                      child: Padding(
                        padding: EdgeInsets.all(24),
                        child: CircularProgressIndicator(
                          key: Key('revision3-npc-profile-edit-loading'),
                        ),
                      ),
                    ),
                  )
                else if (checkpoint != null) ...[
                  TextField(
                    key: const Key('revision3-npc-profile-edit-name'),
                    controller: _name,
                    enabled: enabled,
                    maxLength: 256,
                    inputFormatters: [LengthLimitingTextInputFormatter(256)],
                    textInputAction: TextInputAction.done,
                    decoration: InputDecoration(
                      labelText: widget.copy.nameLabel,
                      helperText: widget.copy.nameHint,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    key: ValueKey(
                      'revision3-npc-profile-edit-archetype-${checkpoint.catalog.npcCatalogSeal?.sha256}',
                    ),
                    initialValue: _archetypeId,
                    isExpanded: true,
                    decoration: InputDecoration(
                      labelText: widget.copy.archetypeLabel,
                      helperText: widget.copy.archetypeHelp,
                      helperMaxLines: 3,
                      border: const OutlineInputBorder(),
                    ),
                    items: [
                      for (final choice in checkpoint.catalog.choices)
                        DropdownMenuItem<String>(
                          value: choice.catalogId,
                          child: Text(
                            choice.displayName,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                    ],
                    onChanged: enabled
                        ? (value) => setState(() {
                            _archetypeId = value;
                            _reviewRequired = false;
                            _error = null;
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
                      key: const Key('revision3-npc-profile-edit-error'),
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
          if (!_loading && checkpoint == null && !_locked)
            TextButton.icon(
              key: const Key('revision3-npc-profile-edit-retry'),
              onPressed: _busy ? null : _load,
              icon: const Icon(Icons.refresh),
              label: Text(widget.copy.retry),
            ),
          TextButton(
            key: const Key('revision3-npc-profile-edit-cancel'),
            onPressed: _busy ? null : _cancel,
            child: Text(_locked ? widget.copy.close : widget.copy.cancel),
          ),
          FilledButton.icon(
            key: const Key('revision3-npc-profile-edit-save'),
            onPressed: _canSave ? _save : null,
            icon: _busy
                ? const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.save_outlined),
            label: Text(_busy ? widget.copy.saving : widget.copy.save),
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
      barrierDismissible: false,
      builder: (context) => AlertDialog(
        key: const Key('revision3-npc-profile-edit-discard-dialog'),
        title: Text(widget.copy.discardTitle),
        content: Text(widget.copy.discardBody),
        actions: [
          TextButton(
            key: const Key('revision3-npc-profile-edit-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(widget.copy.keepEditing),
          ),
          FilledButton(
            key: const Key('revision3-npc-profile-edit-discard'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(widget.copy.discard),
          ),
        ],
      ),
    );
    if (!mounted || discard != true) return;
    await _popAfterUnlock();
  }

  Future<void> _popAfterUnlock([
    Revision3NpcProfileEditPublication? publication,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(publication);
  }
}
