import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceOggPicker = Future<String?> Function();

/// Visible normal-mode workflow for attaching a real Ogg take to one existing
/// managed-R3 dialog line. It exposes no entity IDs, CAS hashes, build,
/// deployment, game-write, save-write, or runtime controls.
class Revision3VoiceTakeDialog extends StatefulWidget {
  const Revision3VoiceTakeDialog({
    required this.service,
    this.pickOgg,
    this.initialLineId,
    this.initialLocale,
    super.key,
  });

  final Revision3VoiceAuthoringService service;
  final Revision3VoiceOggPicker? pickOgg;

  /// Optional exact-current selection supplied by a preceding project action,
  /// such as creating a new DialogLine. The freshly loaded catalog still has
  /// to contain the line; stale or malformed values are discarded.
  final String? initialLineId;
  final String? initialLocale;

  @override
  State<Revision3VoiceTakeDialog> createState() =>
      _Revision3VoiceTakeDialogState();
}

class _Revision3VoiceTakeDialogState extends State<Revision3VoiceTakeDialog> {
  final _formKey = GlobalKey<FormState>();
  final _locale = TextEditingController();
  final _source = TextEditingController();
  final _takeName = TextEditingController();

  Revision3VoiceCatalog? _catalog;
  String? _lineId;
  AuthoringRevision3VoiceTakeStatus _status =
      AuthoringRevision3VoiceTakeStatus.recorded;
  bool _selectTake = false;
  bool _replacementConfirmed = false;
  bool _takeNameWasManuallyEdited = false;
  bool _loading = true;
  bool _publishing = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  bool _picking = false;
  String? _error;
  int _loadGeneration = 0;
  int _catalogEpoch = 0;

  @override
  void initState() {
    super.initState();
    _lineId = widget.initialLineId;
    _locale.text = widget.initialLocale ?? '';
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTakeDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service)) {
      _loadCatalog(clear: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    _locale.dispose();
    _source.dispose();
    _takeName.dispose();
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _lineId = null;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalog = catalog;
        _catalogEpoch += 1;
        if (catalog.line(_lineId ?? '') == null) {
          _lineId = null;
          _selectTake = false;
          _replacementConfirmed = false;
        }
        if (_locale.text.isEmpty) {
          _locale.text = catalog.suggestedLocales.first;
        }
        _loading = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _error =
            'Dialog lines could not be read from the exact current project. No project, game, or save files were changed.';
      });
    }
  }

  Future<void> _pickSource() async {
    final picker = widget.pickOgg ?? _pickRevision3VoiceOgg;
    if (_picking || _publishing) return;
    setState(() {
      _picking = true;
      _error = null;
    });
    try {
      final path = await picker();
      if (!mounted || path == null) return;
      setState(() {
        _source.text = path;
        if (!_takeNameWasManuallyEdited) {
          final leaf = path.replaceAll('\\', '/').split('/').last;
          _takeName.text = leaf.toLowerCase().endsWith('.ogg')
              ? leaf.substring(0, leaf.length - 4)
              : leaf;
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The Ogg picker could not be opened. You can enter the file path manually.';
      });
    } finally {
      if (mounted) setState(() => _picking = false);
    }
  }

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _catalog?.line(_lineId ?? '');

  Revision3VoiceExistingSlotSummary? get _selectedSlotSummary =>
      _selectedLine?.slotSummaryForLocale(_locale.text.trim());

  bool get _selectedLocaleBlocked {
    final line = _selectedLine;
    final locale = _locale.text.trim();
    return line != null &&
        revision3VoiceLocaleIsCanonical(locale) &&
        !line.isLocaleAuthorable(locale);
  }

  bool get _replacementConfirmationRequired =>
      _selectTake &&
      _status == AuthoringRevision3VoiceTakeStatus.approved &&
      (_selectedSlotSummary?.hasSelectedTake ?? false);

  void _selectLine(Revision3VoiceDialogLineChoice line) {
    setState(() {
      _lineId = line.lineId;
      _selectTake = false;
      _replacementConfirmed = false;
    });
  }

  void _clearChangedLineSearch(String value) {
    final selected = _selectedLine;
    if (selected != null && value != selected.displayLabel) {
      setState(() {
        _lineId = null;
        _selectTake = false;
        _replacementConfirmed = false;
      });
    }
  }

  void _changeLocale(String value) {
    setState(() {
      _locale.text = value;
      _selectTake = false;
      _replacementConfirmed = false;
    });
  }

  Future<void> _submit() async {
    final catalog = _catalog;
    final lineId = _lineId;
    if (_publishing ||
        _requiresReopen ||
        _staleCheckpoint ||
        catalog == null ||
        lineId == null ||
        _selectedLocaleBlocked ||
        (_replacementConfirmationRequired && !_replacementConfirmed) ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }

    final Revision3VoiceTakeAuthoringInput input;
    try {
      input = Revision3VoiceTakeAuthoringInput(
        lineId: lineId,
        locale: _locale.text,
        sourcePath: _source.text,
        takeDisplayName: _takeName.text,
        status: _status,
        selectTake: _selectTake,
        dialogText: null,
      );
    } on FormatException catch (error) {
      setState(() => _error = error.message.toString());
      return;
    }

    setState(() {
      _publishing = true;
      _publicationStarted = true;
      _error = null;
    });
    var completed = false;
    try {
      final publication = await widget.service.publish(
        checkpoint: catalog,
        input: input,
      );
      if (!mounted) return;
      completed = true;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTakeRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _error =
            'This project can no longer be verified as current. Close this window and reopen the managed project before continuing.';
      });
    } on Revision3VoiceTakeStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _error =
            'The managed project changed while this window was open. Close it and add the take again from the current project.';
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() => _error = _voiceImportErrorMessage(error.code));
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The Voice take could not be saved. Nothing was built, deployed, or written into the game or a save. Review the form and try again.';
      });
    } finally {
      if (mounted && !completed) {
        setState(() {
          _publishing = false;
          _publicationStarted = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final blocked = _requiresReopen || _staleCheckpoint;
    final busy = _loading || _publishing || _picking;
    return PopScope(
      canPop: !_publicationStarted,
      child: AlertDialog(
        key: const Key('revision3-voice-wizard'),
        title: const Row(
          children: [
            Icon(Icons.record_voice_over_outlined),
            SizedBox(width: 10),
            Expanded(child: Text('Add a Voice take')),
          ],
        ),
        content: SizedBox(
          width: 680,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 680),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const _VoiceBoundaryBanner(),
                  const SizedBox(height: 16),
                  if (_publishing) ...[
                    const _VoiceLiveStatus(
                      key: Key('revision3-voice-saving-status'),
                      message: 'Saving Voice take to the managed project…',
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_error case final error?) ...[
                    _VoiceMessage(
                      key: const Key('revision3-voice-error'),
                      message: error,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_loading)
                    Semantics(
                      liveRegion: true,
                      label: 'Loading dialog lines from the current project',
                      child: const Padding(
                        padding: EdgeInsets.symmetric(vertical: 40),
                        child: Center(
                          child: CircularProgressIndicator(
                            key: Key('revision3-voice-loading'),
                          ),
                        ),
                      ),
                    )
                  else if (_catalog == null)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-voice-retry'),
                        onPressed: _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: const Text('Refresh dialog lines'),
                      ),
                    )
                  else
                    _buildForm(enabled: !busy && !blocked),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-cancel'),
            onPressed: _publicationStarted
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(blocked ? 'Close' : 'Cancel'),
          ),
          FilledButton.icon(
            key: const Key('revision3-voice-submit'),
            onPressed:
                busy ||
                    _catalog == null ||
                    _lineId == null ||
                    blocked ||
                    _selectedLocaleBlocked ||
                    (_replacementConfirmationRequired && !_replacementConfirmed)
                ? null
                : _submit,
            icon: _publishing
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.library_add_outlined),
            label: Text(_publishing ? 'Saving take…' : 'Add take to project'),
          ),
        ],
      ),
    );
  }

  Widget _buildForm({required bool enabled}) {
    final catalog = _catalog!;
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          RawAutocomplete<Revision3VoiceDialogLineChoice>(
            key: ValueKey('revision3-voice-line-$_catalogEpoch'),
            initialValue: TextEditingValue(
              text: _selectedLine?.displayLabel ?? '',
            ),
            displayStringForOption: (line) => line.displayLabel,
            optionsBuilder: (value) {
              final query = value.text.trim();
              if (query.isEmpty) {
                return const <Revision3VoiceDialogLineChoice>[];
              }
              return catalog.lines
                  .where((line) => line.matches(query))
                  .take(50);
            },
            onSelected: _selectLine,
            fieldViewBuilder:
                (
                  context,
                  controller,
                  focusNode,
                  onFieldSubmitted,
                ) => TextFormField(
                  key: const Key('revision3-voice-line-search'),
                  controller: controller,
                  focusNode: focusNode,
                  enabled: enabled,
                  decoration: const InputDecoration(
                    labelText: 'Dialog line',
                    hintText: 'Search by speaker, line name, or Loc ID',
                    helperText:
                        'Type to search, then choose one exact existing line.',
                    border: OutlineInputBorder(),
                  ),
                  onChanged: _clearChangedLineSearch,
                  onFieldSubmitted: (_) => onFieldSubmitted(),
                  validator: (_) => _lineId == null
                      ? 'Search for and choose a dialog line'
                      : null,
                ),
            optionsViewBuilder: (context, onSelected, options) {
              final bounded = options.toList(growable: false);
              return Align(
                alignment: Alignment.topLeft,
                child: Material(
                  elevation: 6,
                  clipBehavior: Clip.antiAlias,
                  borderRadius: BorderRadius.circular(8),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(
                      maxWidth: 640,
                      maxHeight: 300,
                    ),
                    child: ListView.builder(
                      key: const Key('revision3-voice-line-results'),
                      padding: EdgeInsets.zero,
                      shrinkWrap: true,
                      itemCount: bounded.length,
                      itemBuilder: (context, index) {
                        final line = bounded[index];
                        return ListTile(
                          title: Text(line.displayLabel),
                          onTap: () => onSelected(line),
                        );
                      },
                    ),
                  ),
                ),
              );
            },
          ),
          const SizedBox(height: 14),
          TextFormField(
            key: const Key('revision3-voice-locale'),
            controller: _locale,
            enabled: enabled,
            maxLength: 35,
            decoration: const InputDecoration(
              labelText: 'Language code',
              hintText: 'de',
              helperText: 'Examples: de, en, en-US',
              border: OutlineInputBorder(),
            ),
            validator: _validateLocale,
            onChanged: (value) {
              setState(() {
                _selectTake = false;
                _replacementConfirmed = false;
              });
            },
          ),
          if (catalog.suggestedLocales.isNotEmpty) ...[
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: [
                for (final locale in catalog.suggestedLocales)
                  ChoiceChip(
                    key: Key('revision3-voice-locale-$locale'),
                    label: Text(locale),
                    selected: _locale.text.trim() == locale,
                    onSelected: enabled
                        ? (selected) {
                            if (selected && _locale.text != locale) {
                              _changeLocale(locale);
                            }
                          }
                        : null,
                  ),
              ],
            ),
            const SizedBox(height: 14),
          ],
          if (_selectedLine != null &&
              revision3VoiceLocaleIsCanonical(_locale.text.trim())) ...[
            _VoiceSlotSummary(
              summary: _selectedSlotSummary,
              blocked: _selectedLocaleBlocked,
            ),
            const SizedBox(height: 14),
          ],
          TextFormField(
            key: const Key('revision3-voice-source'),
            controller: _source,
            enabled: enabled,
            decoration: InputDecoration(
              labelText: 'Ogg recording',
              helperText:
                  'Vorbis and Opus Ogg files are validated before the project changes.',
              border: const OutlineInputBorder(),
              suffixIcon: IconButton(
                key: const Key('revision3-voice-browse'),
                tooltip: 'Choose Ogg file',
                onPressed: enabled ? _pickSource : null,
                icon: const Icon(Icons.folder_open),
              ),
            ),
            validator: _validateSource,
          ),
          const SizedBox(height: 14),
          TextFormField(
            key: const Key('revision3-voice-take-name'),
            controller: _takeName,
            enabled: enabled,
            maxLength: 256,
            decoration: const InputDecoration(
              labelText: 'Take name',
              hintText: 'Asghan German take 1',
              helperText:
                  'Initially suggested from the Ogg file name; you can rename it.',
              border: OutlineInputBorder(),
            ),
            onChanged: (_) => _takeNameWasManuallyEdited = true,
            validator: (value) => (value?.trim().isEmpty ?? true)
                ? 'Enter a take name'
                : utf8.encode(value!.trim()).length > 256
                ? 'Take name is too long'
                : null,
          ),
          const SizedBox(height: 14),
          DropdownButtonFormField<AuthoringRevision3VoiceTakeStatus>(
            key: const Key('revision3-voice-status'),
            initialValue: _status,
            decoration: const InputDecoration(
              labelText: 'Review status',
              helperText:
                  'Manually set metadata; audio is not reviewed or approved automatically.',
              border: OutlineInputBorder(),
            ),
            items: const [
              DropdownMenuItem(
                value: AuthoringRevision3VoiceTakeStatus.draft,
                child: Text('Draft'),
              ),
              DropdownMenuItem(
                value: AuthoringRevision3VoiceTakeStatus.recorded,
                child: Text('Recorded'),
              ),
              DropdownMenuItem(
                value: AuthoringRevision3VoiceTakeStatus.reviewed,
                child: Text('Reviewed'),
              ),
              DropdownMenuItem(
                value: AuthoringRevision3VoiceTakeStatus.approved,
                child: Text('Approved'),
              ),
            ],
            onChanged: enabled
                ? (value) {
                    if (value == null) return;
                    setState(() {
                      _status = value;
                      if (value != AuthoringRevision3VoiceTakeStatus.approved) {
                        _selectTake = false;
                      }
                      _replacementConfirmed = false;
                    });
                  }
                : null,
          ),
          CheckboxListTile(
            key: const Key('revision3-voice-select'),
            contentPadding: EdgeInsets.zero,
            value: _selectTake,
            title: const Text('Use this as the selected take'),
            subtitle: const Text(
              'Available only after marking the take Approved.',
            ),
            onChanged:
                enabled && _status == AuthoringRevision3VoiceTakeStatus.approved
                ? (value) => setState(() {
                    _selectTake = value ?? false;
                    _replacementConfirmed = false;
                  })
                : null,
          ),
          if (_replacementConfirmationRequired) ...[
            const SizedBox(height: 8),
            _VoiceReplacementWarning(
              confirmed: _replacementConfirmed,
              enabled: enabled,
              onChanged: (value) =>
                  setState(() => _replacementConfirmed = value),
            ),
          ],
          const Divider(height: 24),
          const _VoiceLocalizationPreservedNotice(),
        ],
      ),
    );
  }
}

class _VoiceSlotSummary extends StatelessWidget {
  const _VoiceSlotSummary({required this.summary, required this.blocked});

  final Revision3VoiceExistingSlotSummary? summary;
  final bool blocked;

  @override
  Widget build(BuildContext context) {
    final value = summary;
    final message = blocked
        ? 'This line already has a Voice slot for this language, but its project graph is not safe to extend. Choose another language or repair the project first.'
        : value == null
        ? 'No Voice slot exists for this line and language yet. One will be added with the take.'
        : '${value.candidateCount} existing take${value.candidateCount == 1 ? '' : 's'} · ${value.hasSelectedTake ? 'A take is currently selected.' : 'No take is currently selected.'}';
    return Container(
      key: const Key('revision3-voice-slot-summary'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.library_music_outlined, size: 20),
          const SizedBox(width: 10),
          Expanded(child: Text(message)),
        ],
      ),
    );
  }
}

class _VoiceReplacementWarning extends StatelessWidget {
  const _VoiceReplacementWarning({
    required this.confirmed,
    required this.enabled,
    required this.onChanged,
  });

  final bool confirmed;
  final bool enabled;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-replacement-warning'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: scheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'A take is already selected',
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(color: scheme.onErrorContainer),
          ),
          const SizedBox(height: 4),
          Text(
            'Selecting this approved take will replace the current selection for this dialog line and language.',
            style: TextStyle(color: scheme.onErrorContainer),
          ),
          Material(
            type: MaterialType.transparency,
            child: CheckboxListTile(
              key: const Key('revision3-voice-confirm-replacement'),
              contentPadding: EdgeInsets.zero,
              value: confirmed,
              title: const Text('I understand and want to replace it'),
              controlAffinity: ListTileControlAffinity.leading,
              onChanged: enabled ? (value) => onChanged(value ?? false) : null,
            ),
          ),
        ],
      ),
    );
  }
}

class _VoiceLocalizationPreservedNotice extends StatelessWidget {
  const _VoiceLocalizationPreservedNotice();

  @override
  Widget build(BuildContext context) => const ListTile(
    key: Key('revision3-voice-localization-preserved'),
    contentPadding: EdgeInsets.zero,
    leading: Icon(Icons.lock_outline),
    title: Text('Existing dialog text is preserved'),
    subtitle: Text(
      'Text editing is unavailable here until the current language text can be displayed and verified safely.',
    ),
  );
}

class _VoiceBoundaryBanner extends StatelessWidget {
  const _VoiceBoundaryBanner();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-boundary'),
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
              Chip(label: Text('Saved to project only')),
              Chip(label: Text('Not yet usable in game')),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'This imports one real Ogg recording into the managed project. It does not compile, deploy, modify game files, or touch a save.',
            style: TextStyle(color: scheme.onSecondaryContainer),
          ),
        ],
      ),
    );
  }
}

class _VoiceLiveStatus extends StatelessWidget {
  const _VoiceLiveStatus({required this.message, super.key});

  final String message;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: Row(
      children: [
        const SizedBox.square(
          dimension: 18,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
        const SizedBox(width: 10),
        Expanded(child: Text(message)),
      ],
    ),
  );
}

class _VoiceMessage extends StatelessWidget {
  const _VoiceMessage({required this.message, super.key});

  final String message;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: scheme.errorContainer,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Text(message, style: TextStyle(color: scheme.onErrorContainer)),
      ),
    );
  }
}

String? _validateLocale(String? value) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty) return 'Enter a language code';
  if (!revision3VoiceLocaleIsCanonical(normalized)) {
    return 'Use a language code such as de or en-US';
  }
  return null;
}

String _voiceImportErrorMessage(String code) => switch (code) {
  'AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE' =>
    'The configured Gothic 1 Remake installation is unavailable. Check it in Settings, then try again.',
  'AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS' =>
    'This project folder overlaps the configured game installation. Move the project outside the game folder before adding a Voice take.',
  'AUTHORING_REVISION3_VOICE_INPUT_MISSING' =>
    'The selected Ogg file no longer exists. Choose the recording again.',
  'AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE' =>
    'The selected Ogg file could not be read. Close any app that is holding it, then try again.',
  'AUTHORING_REVISION3_VOICE_INPUT_UNSAFE' =>
    'The selected source could not be opened safely. Choose a regular local .ogg file.',
  'AUTHORING_REVISION3_VOICE_INPUT_LIMIT' =>
    'The selected Ogg file is larger than the supported import limit.',
  'AUTHORING_REVISION3_VOICE_OGG_INVALID' =>
    'The selected file is not a supported, valid Vorbis or Opus Ogg recording.',
  'AUTHORING_REVISION3_VOICE_INPUT_CHANGED' =>
    'The Ogg file changed while it was being verified. Wait for the recording to finish, then choose it again.',
  'AUTHORING_REVISION3_VOICE_LIMIT' =>
    'This project cannot accept another Voice take at its current capacity.',
  'AUTHORING_REVISION3_VOICE_INTENT_INVALID' ||
  'AUTHORING_REVISION3_VOICE_STATUS_INVALID' =>
    'The Voice take details are no longer valid for this line. Review the form and try again.',
  _ =>
    'The Voice take could not be saved. Nothing was built, deployed, or written into the game or a save. Review the form and try again.',
};

Future<String?> _pickRevision3VoiceOgg() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'Ogg audio', extensions: ['ogg']),
    ],
  );
  return file?.path;
}

String? _validateSource(String? value) {
  final source = value ?? '';
  if (source.isEmpty) return 'Choose an Ogg recording';
  if (source.trim() != source || !source.toLowerCase().endsWith('.ogg')) {
    return 'Choose a file ending in .ogg';
  }
  return null;
}
