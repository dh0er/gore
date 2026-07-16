import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

/// Normal-mode workflow for refreshing the installed-archive evidence of one
/// existing, structurally safe managed-R3 Voice slot.
///
/// This dialog cannot create slots, edit technical identities, build, deploy,
/// modify game files, or touch a save. The selected line and locale always
/// originate from one exact content-index checkpoint.
class Revision3VoiceTargetDialog extends StatefulWidget {
  const Revision3VoiceTargetDialog({
    required this.service,
    this.initialLineId,
    this.initialLocale,
    this.fixedContext = false,
    super.key,
  });

  final Revision3VoiceTargetAuthoringService service;
  final String? initialLineId;
  final String? initialLocale;

  /// Keeps an in-workspace line/locale handoff fixed. The freshly loaded
  /// catalog must still prove the exact existing slot targetable.
  final bool fixedContext;

  @override
  State<Revision3VoiceTargetDialog> createState() =>
      _Revision3VoiceTargetDialogState();
}

class _Revision3VoiceTargetDialogState
    extends State<Revision3VoiceTargetDialog> {
  Revision3VoiceCatalog? _catalog;
  List<Revision3VoiceDialogLineChoice> _lines = const [];
  String? _lineId;
  String? _locale;
  bool _loading = true;
  bool _resolving = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  bool _fixedContextInvalid = false;
  String? _error;
  int _loadGeneration = 0;
  int _catalogEpoch = 0;
  bool _initialSelectionConsumed = false;

  @override
  void initState() {
    super.initState();
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTargetDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service) ||
        oldWidget.fixedContext != widget.fixedContext ||
        oldWidget.initialLineId != widget.initialLineId ||
        oldWidget.initialLocale != widget.initialLocale) {
      _initialSelectionConsumed = false;
      _loadCatalog(clear: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _fixedContextInvalid = false;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      final lines = catalog.lines
          .where((line) => _safeLocales(line).isNotEmpty)
          .toList(growable: false);
      setState(() {
        _catalog = catalog;
        _lines = List.unmodifiable(lines);
        _catalogEpoch += 1;
        if (widget.fixedContext) {
          final requestedLine = _lineFrom(lines, widget.initialLineId);
          final requestedLocale = widget.initialLocale;
          if (requestedLine != null &&
              requestedLocale != null &&
              _safeLocales(requestedLine).contains(requestedLocale)) {
            _lineId = requestedLine.lineId;
            _locale = requestedLocale;
            _fixedContextInvalid = false;
          } else {
            _lineId = null;
            _locale = null;
            _fixedContextInvalid = true;
            _error =
                'This Voice action no longer matches one intact existing Voice target in the exact current project. Close it and reopen Resolve target from the current workspace. No project, game, or save files were changed.';
          }
        } else {
          final initial = _initialSelectionConsumed
              ? null
              : _lineFrom(lines, widget.initialLineId);
          _initialSelectionConsumed = true;
          final selected = initial ?? _lineFrom(lines, _lineId);
          if (selected == null) {
            _lineId = null;
            _locale = null;
          } else {
            _lineId = selected.lineId;
            final locales = _safeLocales(selected);
            final requestedLocale = initial == null
                ? _locale
                : widget.initialLocale;
            _locale = locales.contains(requestedLocale)
                ? requestedLocale
                : locales.first;
          }
        }
        _loading = false;
      });
    } on Revision3VoiceTargetRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _requiresReopen = true;
        _error =
            'This project can no longer be verified as current. Close this window and reopen the managed project before resolving another Voice target.';
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _error =
            'Existing Voice slots could not be read from the exact current project. No project, game, or save files were changed.';
      });
    }
  }

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _lineFrom(_lines, _lineId);

  Revision3VoiceExistingSlotSummary? get _selectedSummary {
    final line = _selectedLine;
    final locale = _locale;
    return line == null || locale == null
        ? null
        : line.slotSummaryForLocale(locale);
  }

  bool get _fixedContextIsCurrent {
    if (!widget.fixedContext) return true;
    final line = _selectedLine;
    final locale = _locale;
    return !_fixedContextInvalid &&
        line != null &&
        line.lineId == widget.initialLineId &&
        locale != null &&
        locale == widget.initialLocale &&
        _safeLocales(line).contains(locale);
  }

  void _selectLine(Revision3VoiceDialogLineChoice line) {
    final locales = _safeLocales(line);
    if (locales.isEmpty) return;
    setState(() {
      _lineId = line.lineId;
      _locale = locales.first;
      _error = null;
    });
  }

  void _clearChangedLineSearch(String value) {
    final selected = _selectedLine;
    if (selected != null && value != selected.displayLabel) {
      setState(() {
        _lineId = null;
        _locale = null;
      });
    }
  }

  Future<void> _resolve() async {
    final catalog = _catalog;
    final line = _selectedLine;
    final locale = _locale;
    if (_resolving ||
        _requiresReopen ||
        _staleCheckpoint ||
        catalog == null ||
        line == null ||
        locale == null ||
        !_fixedContextIsCurrent ||
        !_safeLocales(line).contains(locale)) {
      return;
    }
    setState(() {
      _resolving = true;
      _publicationStarted = true;
      _error = null;
    });
    var completed = false;
    try {
      final publication = await widget.service.resolve(
        checkpoint: catalog,
        lineId: line.lineId,
        locale: locale,
      );
      if (!mounted) return;
      completed = true;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTargetRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _error =
            'This project can no longer be verified as current. Close this window and reopen the managed project before resolving another Voice target.';
      });
    } on Revision3VoiceTargetStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _error =
            'The managed project changed while this window was open. Close this resolver and open it again from the current project.';
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() => _error = _voiceTargetErrorMessage(error.code));
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The installed Voice target could not be resolved. No bundle was built, nothing was deployed, and no game or save file was changed. Check the installation and try again.';
      });
    } finally {
      if (mounted && !completed) {
        setState(() {
          _resolving = false;
          _publicationStarted = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final blocked = _requiresReopen || _staleCheckpoint;
    final canResolve =
        !_loading &&
        !_resolving &&
        !blocked &&
        _fixedContextIsCurrent &&
        _catalog != null &&
        _selectedLine != null &&
        _locale != null;
    return PopScope(
      canPop: !_publicationStarted,
      child: AlertDialog(
        key: const Key('revision3-voice-target-dialog'),
        title: const Row(
          children: [
            Icon(Icons.manage_search_outlined),
            SizedBox(width: 10),
            Expanded(child: Text('Resolve installed Voice target')),
          ],
        ),
        content: SizedBox(
          width: 680,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 640),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const _TargetBoundaryBanner(),
                  const SizedBox(height: 16),
                  if (_resolving) ...[
                    const _TargetLiveStatus(
                      key: Key('revision3-voice-target-resolving'),
                      message:
                          'Checking the installed Voice archive and saving exact evidence...',
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_error case final message?) ...[
                    _TargetMessage(
                      key: const Key('revision3-voice-target-error'),
                      message: message,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_loading)
                    const Padding(
                      padding: EdgeInsets.symmetric(vertical: 40),
                      child: Center(
                        child: CircularProgressIndicator(
                          key: Key('revision3-voice-target-loading'),
                        ),
                      ),
                    )
                  else if (_catalog == null)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-voice-target-retry'),
                        onPressed: blocked ? null : _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: const Text('Refresh existing Voice slots'),
                      ),
                    )
                  else if (_fixedContextInvalid)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key(
                          'revision3-voice-target-fixed-context-retry',
                        ),
                        onPressed: blocked ? null : _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: const Text('Refresh Voice context'),
                      ),
                    )
                  else if (_lines.isEmpty)
                    const _NoSafeVoiceSlots()
                  else
                    _buildSelection(enabled: !_resolving && !blocked),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-target-cancel'),
            onPressed: _publicationStarted
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(blocked ? 'Close' : 'Cancel'),
          ),
          FilledButton.icon(
            key: const Key('revision3-voice-target-submit'),
            onPressed: canResolve ? _resolve : null,
            icon: _resolving
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.travel_explore_outlined),
            label: Text(
              _resolving ? 'Resolving target...' : 'Resolve installed target',
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSelection({required bool enabled}) {
    final line = _selectedLine;
    final locales = line == null ? const <String>[] : _safeLocales(line);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (widget.fixedContext)
          _TargetFixedContextBreadcrumb(
            lineLabel: line!.displayLabel,
            locale: _locale!,
          )
        else
          RawAutocomplete<Revision3VoiceDialogLineChoice>(
            key: ValueKey('revision3-voice-target-line-$_catalogEpoch'),
            initialValue: TextEditingValue(text: line?.displayLabel ?? ''),
            displayStringForOption: (option) => option.displayLabel,
            optionsBuilder: (value) {
              final query = value.text.trim();
              if (query.isEmpty) {
                return const <Revision3VoiceDialogLineChoice>[];
              }
              return _lines.where((option) => option.matches(query)).take(50);
            },
            onSelected: _selectLine,
            fieldViewBuilder:
                (
                  context,
                  controller,
                  focusNode,
                  onFieldSubmitted,
                ) => TextFormField(
                  key: const Key('revision3-voice-target-line-search'),
                  controller: controller,
                  focusNode: focusNode,
                  enabled: enabled,
                  decoration: const InputDecoration(
                    labelText: 'Dialog line with an existing Voice slot',
                    hintText: 'Search by speaker, line name, or Loc ID',
                    helperText:
                        'Only intact lines that already own a safe Voice slot are shown.',
                    border: OutlineInputBorder(),
                  ),
                  onChanged: _clearChangedLineSearch,
                  onFieldSubmitted: (_) => onFieldSubmitted(),
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
                      key: const Key('revision3-voice-target-line-results'),
                      padding: EdgeInsets.zero,
                      shrinkWrap: true,
                      itemCount: bounded.length,
                      itemBuilder: (context, index) {
                        final option = bounded[index];
                        return ListTile(
                          title: Text(option.displayLabel),
                          onTap: () => onSelected(option),
                        );
                      },
                    ),
                  ),
                ),
              );
            },
          ),
        const SizedBox(height: 14),
        if (!widget.fixedContext)
          DropdownButtonFormField<String>(
            key: ValueKey(
              'revision3-voice-target-locale-${line?.lineId ?? 'none'}',
            ),
            initialValue: locales.contains(_locale) ? _locale : null,
            decoration: const InputDecoration(
              labelText: 'Existing Voice-slot language',
              helperText:
                  'Only languages with an intact existing slot can be resolved here.',
              border: OutlineInputBorder(),
            ),
            items: [
              for (final locale in locales)
                DropdownMenuItem(value: locale, child: Text(locale)),
            ],
            onChanged: enabled && line != null
                ? (value) => setState(() {
                    _locale = value;
                    _error = null;
                  })
                : null,
          ),
        if (_selectedSummary case final summary?) ...[
          const SizedBox(height: 14),
          _CurrentTargetState(summary: summary),
        ],
        const SizedBox(height: 14),
        const _TargetOutcomeNotice(),
      ],
    );
  }
}

class _TargetFixedContextBreadcrumb extends StatelessWidget {
  const _TargetFixedContextBreadcrumb({
    required this.lineLabel,
    required this.locale,
  });

  final String lineLabel;
  final String locale;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-target-fixed-context'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.subdirectory_arrow_right, size: 20),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(lineLabel, style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 3),
              Text('Voice language: $locale'),
            ],
          ),
        ),
      ],
    ),
  );
}

List<String> _safeLocales(Revision3VoiceDialogLineChoice line) {
  final locales =
      line.existingSlotLocales
          .where(
            (locale) =>
                line.isLocaleTargetable(locale) &&
                line.slotSummaryForLocale(locale) != null,
          )
          .toList(growable: false)
        ..sort();
  return locales;
}

Revision3VoiceDialogLineChoice? _lineFrom(
  List<Revision3VoiceDialogLineChoice> lines,
  String? id,
) {
  if (id == null) return null;
  for (final line in lines) {
    if (line.lineId == id) return line;
  }
  return null;
}

class _CurrentTargetState extends StatelessWidget {
  const _CurrentTargetState({required this.summary});

  final Revision3VoiceExistingSlotSummary summary;

  @override
  Widget build(BuildContext context) {
    final (icon, title, message) = switch (summary.targetResolution) {
      Revision3ContentVoiceTargetResolution.unresolved => (
        Icons.link_off_outlined,
        'Current target: unresolved',
        'No exact installed archive member is currently linked to this slot.',
      ),
      Revision3ContentVoiceTargetResolution.ambiguous => (
        Icons.call_split_outlined,
        'Current target: ambiguous',
        'Multiple installed archive members matched previously; no member was chosen implicitly.',
      ),
      Revision3ContentVoiceTargetResolution.resolved => (
        Icons.verified_outlined,
        'Current target: resolved',
        'One exact installed archive member is currently sealed for this slot. Resolving again refreshes that evidence.',
      ),
    };
    return Container(
      key: const Key('revision3-voice-target-current-state'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 3),
                Text(message),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _TargetBoundaryBanner extends StatelessWidget {
  const _TargetBoundaryBanner();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-target-boundary'),
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
              Chip(label: Text('Saves evidence to project')),
              Chip(label: Text('Does not deploy')),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'This checks the installed Voice archive for one existing slot and saves only exact match evidence. It does not change the archive, build a mod, deploy, or touch a save.',
            style: TextStyle(color: scheme.onSecondaryContainer),
          ),
        ],
      ),
    );
  }
}

class _TargetOutcomeNotice extends StatelessWidget {
  const _TargetOutcomeNotice();

  @override
  Widget build(BuildContext context) => const ListTile(
    key: Key('revision3-voice-target-outcome-notice'),
    contentPadding: EdgeInsets.zero,
    leading: Icon(Icons.rule_folder_outlined),
    title: Text('No match is invented'),
    subtitle: Text(
      'Zero, one, or multiple exact matches are saved honestly as unresolved, resolved, or ambiguous.',
    ),
  );
}

class _NoSafeVoiceSlots extends StatelessWidget {
  const _NoSafeVoiceSlots();

  @override
  Widget build(BuildContext context) => const ListTile(
    key: Key('revision3-voice-target-empty'),
    contentPadding: EdgeInsets.symmetric(vertical: 24),
    leading: Icon(Icons.info_outline),
    title: Text('No existing safe Voice slot is available'),
    subtitle: Text(
      'Add or repair a Voice slot in the managed project first, then reopen this resolver.',
    ),
  );
}

class _TargetLiveStatus extends StatelessWidget {
  const _TargetLiveStatus({required this.message, super.key});

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

class _TargetMessage extends StatelessWidget {
  const _TargetMessage({required this.message, super.key});

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

String _voiceTargetErrorMessage(String code) => switch (code) {
  'AUTHORING_REVISION3_VOICE_TARGET_GAME_ROOT_UNAVAILABLE' =>
    'The configured Gothic 1 Remake installation is unavailable. Check it in Settings, then try again.',
  'AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS' =>
    'This project folder overlaps the configured game installation. Move the project outside the game folder before resolving Voice targets.',
  'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE' =>
    'The installed game executable could not be read. Finish any game update, check the configured installation, then try again.',
  'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH' =>
    'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before resolving Voice targets.',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE' =>
    'The installed Voice archive for this language is unavailable. Finish any game update, check the installation, then try again.',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE' =>
    'The installed Voice archive could not be opened safely. Repair or verify the game installation before trying again.',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_INVALID' =>
    'The installed Voice archive is invalid or unsupported. Verify the game installation before trying again.',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT' =>
    'The installed Voice archive exceeds the supported safe inspection limits.',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED' =>
    'The Voice archive changed while it was being inspected. Finish the game update, then try again.',
  'AUTHORING_REVISION3_VOICE_TARGET_LOCALE_UNSUPPORTED' =>
    'No supported installed Voice archive is known for this language.',
  'AUTHORING_REVISION3_VOICE_TARGET_MEMBER_INELIGIBLE' =>
    'A matching archive entry exists but is not safe for an exact managed replacement.',
  'AUTHORING_REVISION3_VOICE_TARGET_COLLISION' =>
    'This exact installed Voice target is already owned by another slot in the project.',
  'AUTHORING_REVISION3_VOICE_TARGET_LOC_ID_INVALID' ||
  'AUTHORING_REVISION3_VOICE_TARGET_INTENT_INVALID' ||
  'AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID' =>
    'This Voice slot is no longer eligible for target resolution. Refresh the project and choose it again.',
  _ =>
    'The installed Voice target could not be resolved. No bundle was built, nothing was deployed, and no game or save file was changed. Check the installation and try again.',
};
