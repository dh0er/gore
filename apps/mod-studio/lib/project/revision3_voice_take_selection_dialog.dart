import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_selection_authoring.dart';
import 'revision3_voice_take_status_authoring.dart';

const _clearSelectionValue = '__no_voice_take_selected__';

/// Friendly project-only editor for selecting or clearing an existing Voice
/// take. Entity IDs, CAS heads, paths, and archive internals are never shown.
class Revision3VoiceTakeSelectionDialog extends StatefulWidget {
  const Revision3VoiceTakeSelectionDialog({
    super.key,
    required this.service,
    required this.statusService,
  });

  final Revision3VoiceTakeSelectionAuthoringService service;
  final Revision3VoiceTakeStatusAuthoringService statusService;

  @override
  State<Revision3VoiceTakeSelectionDialog> createState() =>
      _Revision3VoiceTakeSelectionDialogState();
}

class _Revision3VoiceTakeSelectionDialogState
    extends State<Revision3VoiceTakeSelectionDialog> {
  final _searchController = TextEditingController();
  Revision3VoiceCatalog? _catalog;
  String? _lineId;
  String? _locale;
  String? _selectionValue;
  String? _error;
  String? _notice;
  String? _statusBusyTakeId;
  Revision3VoiceTakeStatusPublication? _pendingStatusPublication;
  bool _loading = true;
  bool _busy = false;
  bool _statusWasSaved = false;
  bool _requiresClose = false;
  bool _reloadRequired = false;

  bool get _interactionLocked => _busy || _requiresClose || _reloadRequired;

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_searchChanged);
    _load();
  }

  @override
  void dispose() {
    _searchController
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
      _notice = null;
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted) return;
      setState(() {
        _catalog = catalog;
        _lineId = null;
        _locale = null;
        _selectionValue = null;
        _loading = false;
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = 'Reopen the managed project before changing Voice takes.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = 'Voice takes could not be loaded: ${_friendlyError(error)}';
      });
    }
  }

  List<Revision3VoiceDialogLineChoice> get _visibleLines {
    final catalog = _catalog;
    if (catalog == null) return const [];
    final query = _searchController.text.trim().toLowerCase();
    return catalog.lines
        .where((line) {
          if (_intactLocales(line).isEmpty) return false;
          return query.isEmpty || line.matches(query);
        })
        .toList(growable: false);
  }

  List<String> _intactLocales(Revision3VoiceDialogLineChoice line) => line
      .existingSlotLocales
      .where((locale) => line.slotSummaryForLocale(locale) != null)
      .toList(growable: false);

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _lineId == null ? null : _catalog?.line(_lineId!);

  Revision3VoiceExistingSlotSummary? get _selectedSummary {
    final line = _selectedLine;
    final locale = _locale;
    return line == null || locale == null
        ? null
        : line.slotSummaryForLocale(locale);
  }

  void _chooseLine(Revision3VoiceDialogLineChoice line) {
    final locales = _intactLocales(line);
    final locale = locales.length == 1 ? locales.single : null;
    setState(() {
      _lineId = line.lineId;
      _locale = locale;
      _selectionValue = locale == null
          ? null
          : _selectionValueFor(line.slotSummaryForLocale(locale)!);
      _error = null;
      _notice = null;
    });
  }

  void _chooseLocale(String? locale) {
    final line = _selectedLine;
    setState(() {
      _locale = locale;
      _selectionValue = line == null || locale == null
          ? null
          : _selectionValueFor(line.slotSummaryForLocale(locale)!);
      _error = null;
      _notice = null;
    });
  }

  String _selectionValueFor(Revision3VoiceExistingSlotSummary summary) =>
      summary.selectedTakeId ?? _clearSelectionValue;

  bool get _hasChange {
    final summary = _selectedSummary;
    final value = _selectionValue;
    if (summary == null || value == null) return false;
    return value != _selectionValueFor(summary);
  }

  Future<void> _save() async {
    final catalog = _catalog;
    final lineId = _lineId;
    final locale = _locale;
    final value = _selectionValue;
    if (_interactionLocked ||
        !_hasChange ||
        catalog == null ||
        lineId == null ||
        locale == null ||
        value == null) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      final publication = await widget.service.publish(
        checkpoint: catalog,
        lineId: lineId,
        locale: locale,
        selectedTakeId: value == _clearSelectionValue ? null : value,
      );
      if (!mounted) return;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTakeSelectionStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _requiresClose = true;
        _error =
            'The project changed while this window was open. Close it and try again from the refreshed project.';
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _requiresClose = true;
        _error = 'Reopen the managed project before changing Voice takes.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = 'Voice selection was not saved: ${_friendlyError(error)}';
      });
    }
  }

  Future<void> _changeStatus(
    Revision3VoiceCandidateTake take,
    AuthoringRevision3VoiceTakeStatus desiredStatus,
  ) async {
    final catalog = _catalog;
    final lineId = _lineId;
    final locale = _locale;
    if (_interactionLocked ||
        _hasChange ||
        catalog == null ||
        lineId == null ||
        locale == null) {
      return;
    }
    var publishedThisAttempt = false;
    setState(() {
      _busy = true;
      _statusBusyTakeId = take.id;
      _error = null;
      _notice = null;
    });
    try {
      final publication = await widget.statusService.publish(
        checkpoint: catalog,
        lineId: lineId,
        locale: locale,
        takeId: take.id,
        desiredStatus: desiredStatus,
      );
      publishedThisAttempt = true;
      _statusWasSaved = true;
      _pendingStatusPublication = publication;
      final refreshed = await widget.statusService.loadCatalog();
      if (refreshed.projectId != publication.projectId ||
          refreshed.projectRevision != publication.projectRevision) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      final refreshedLine = refreshed.line(lineId);
      final refreshedSummary = refreshedLine?.slotSummaryForLocale(locale);
      final refreshedTake = refreshedSummary?.candidate(take.id);
      if (refreshedLine == null ||
          refreshedSummary == null ||
          refreshedSummary.slotRevision != publication.slotRevision ||
          refreshedTake == null ||
          refreshedTake.revision != publication.takeRevision ||
          refreshedTake.status.name != publication.status.name) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      if (!mounted) return;
      setState(() {
        _catalog = refreshed;
        _selectionValue = _selectionValueFor(refreshedSummary);
        _busy = false;
        _statusBusyTakeId = null;
        _pendingStatusPublication = null;
        _notice =
            publication.status == AuthoringRevision3VoiceTakeStatus.approved
            ? 'Status changed to Approved. This take can now be selected.'
            : 'Status changed to ${_voiceTakeStatusLabel(publication.status)}.';
      });
    } on Revision3VoiceTakeStatusSelectedTakeException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _error =
            'This take is currently selected. Clear the selection before changing it from Approved.';
      });
    } on Revision3VoiceTakeStatusStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _reloadRequired = true;
        _error =
            'The project changed before this status could be saved. Reload the latest Voice takes before continuing.';
      });
    } on Revision3VoiceTakeStatusRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _requiresClose = true;
        _reloadRequired = false;
        _error = publishedThisAttempt
            ? 'The status was saved, but the latest Voice takes could not be confirmed. Close this window and reopen the managed project.'
            : 'The status result could not be confirmed. Close this window and reopen the managed project before trying again.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? 'The status was saved, but the latest Voice takes could not be confirmed. Reload the takes before continuing; the status change will not be repeated.'
            : 'Voice take status was not saved: ${_friendlyError(error)}';
      });
    }
  }

  Future<void> _reloadTakes() async {
    if (_busy || !_reloadRequired || _requiresClose) return;
    final previous = _catalog;
    final pending = _pendingStatusPublication;
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      final refreshed = await widget.statusService.loadCatalog();
      if (previous == null || refreshed.projectId != previous.projectId) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      if (pending != null &&
          !_catalogConfirmsStatusPublication(refreshed, pending)) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      final line = _lineId == null ? null : refreshed.line(_lineId!);
      final locale =
          line == null ||
              _locale == null ||
              line.slotSummaryForLocale(_locale!) == null
          ? null
          : _locale;
      final summary = locale == null
          ? null
          : line!.slotSummaryForLocale(locale);
      if (!mounted) return;
      setState(() {
        _catalog = refreshed;
        _lineId = line?.lineId;
        _locale = locale;
        _selectionValue = summary == null ? null : _selectionValueFor(summary);
        _pendingStatusPublication = null;
        _reloadRequired = false;
        _busy = false;
        _notice = pending == null
            ? 'Latest Voice takes reloaded.'
            : 'Saved status confirmed from the latest project.';
      });
    } on Revision3VoiceTakeStatusRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error =
            'The latest Voice takes could not be confirmed. Close this window and reopen the managed project.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'Voice takes could not be reloaded: ${_friendlyError(error)} No saved change was repeated.';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final lines = _visibleLines;
    final line = _selectedLine;
    final summary = _selectedSummary;
    return AlertDialog(
      key: const Key('revision3-voice-take-selection-dialog'),
      title: const Text('Manage Voice takes'),
      content: SizedBox(
        width: 760,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text(
                'Choose which existing Approved recording this dialog line should use. Status is an author workflow label only; it does not prove audio quality or in-game readiness. Changes stay in the offline project until separate build and deployment steps.',
              ),
              const SizedBox(height: 16),
              TextField(
                key: const Key('voice-selection-line-search'),
                controller: _searchController,
                enabled: !_loading && !_interactionLocked,
                decoration: const InputDecoration(
                  labelText: 'Find a dialog line',
                  hintText: 'Search by speaker or line name',
                  prefixIcon: Icon(Icons.search),
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 8),
              if (_loading)
                const Center(
                  child: Padding(
                    padding: EdgeInsets.all(24),
                    child: CircularProgressIndicator(),
                  ),
                )
              else if (_catalog != null && lines.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 16),
                  child: Text(
                    'No matching dialog line has an intact existing Voice slot.',
                    key: Key('voice-selection-no-lines'),
                  ),
                )
              else if (_catalog != null)
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 190),
                  child: ListView.builder(
                    key: const Key('voice-selection-lines'),
                    shrinkWrap: true,
                    itemCount: lines.length,
                    itemBuilder: (context, index) {
                      final choice = lines[index];
                      final selected = choice.lineId == _lineId;
                      return Card(
                        color: selected
                            ? theme.colorScheme.secondaryContainer
                            : null,
                        child: ListTile(
                          key: ValueKey('voice-selection-line-$index'),
                          selected: selected,
                          enabled: !_interactionLocked,
                          leading: const Icon(Icons.chat_bubble_outline),
                          title: Text(choice.displayLabel),
                          subtitle: Text(
                            '${_intactLocales(choice).length} Voice language${_intactLocales(choice).length == 1 ? '' : 's'}',
                          ),
                          onTap: _interactionLocked
                              ? null
                              : () => _chooseLine(choice),
                        ),
                      );
                    },
                  ),
                ),
              if (line != null) ...[
                const SizedBox(height: 16),
                DropdownButtonFormField<String>(
                  key: const Key('voice-selection-locale'),
                  initialValue: _locale,
                  decoration: const InputDecoration(
                    labelText: 'Voice language',
                    border: OutlineInputBorder(),
                  ),
                  items: [
                    for (final locale in _intactLocales(line))
                      DropdownMenuItem(value: locale, child: Text(locale)),
                  ],
                  onChanged: _interactionLocked ? null : _chooseLocale,
                ),
              ],
              if (summary != null) ...[
                const SizedBox(height: 18),
                Text('Selected take', style: theme.textTheme.titleMedium),
                const SizedBox(height: 6),
                RadioGroup<String>(
                  groupValue: _selectionValue,
                  onChanged: _interactionLocked
                      ? (_) {}
                      : (value) => setState(() {
                          _selectionValue = value;
                          _error = null;
                        }),
                  child: Column(
                    children: [
                      RadioListTile<String>(
                        key: const Key('voice-selection-clear'),
                        value: _clearSelectionValue,
                        enabled: !_interactionLocked,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('No take selected'),
                        subtitle: Text(
                          summary.selectedTakeId == null
                              ? 'Current selection'
                              : 'Keep the recordings, but clear the active choice',
                        ),
                      ),
                      for (
                        var index = 0;
                        index < summary.candidates.length;
                        index++
                      )
                        _TakeChoiceTile(
                          index: index,
                          take: summary.candidates[index],
                          isCurrent:
                              summary.selectedTakeId ==
                              summary.candidates[index].id,
                          busy: _interactionLocked,
                          statusDisabled: _hasChange,
                          statusBusy:
                              _statusBusyTakeId == summary.candidates[index].id,
                          onStatusChanged: (status) =>
                              _changeStatus(summary.candidates[index], status),
                        ),
                    ],
                  ),
                ),
                if (_selectionValue == _clearSelectionValue &&
                    summary.selectedTakeId != null)
                  Container(
                    key: const Key('voice-selection-clear-warning'),
                    margin: const EdgeInsets.only(top: 8),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      'The takes stay in this project, but Voice build is blocked until an Approved take is selected again.',
                      style: TextStyle(
                        color: theme.colorScheme.onErrorContainer,
                      ),
                    ),
                  ),
                if (_hasChange)
                  const Padding(
                    padding: EdgeInsets.only(top: 8),
                    child: Text(
                      'Save or undo the pending selection before changing a take status.',
                      key: Key('voice-status-selection-pending'),
                    ),
                  ),
                if (summary.candidates.isEmpty)
                  const Padding(
                    padding: EdgeInsets.only(top: 8),
                    child: Text(
                      'This Voice slot has no candidate takes yet.',
                      key: Key('voice-selection-no-candidates'),
                    ),
                  ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 12),
                Text(
                  _error!,
                  key: const Key('voice-selection-error'),
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ],
              if (_notice != null) ...[
                const SizedBox(height: 12),
                Text(
                  _notice!,
                  key: const Key('voice-status-notice'),
                  style: TextStyle(color: theme.colorScheme.primary),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        if (_error != null && _catalog == null && !_loading)
          TextButton(
            key: const Key('voice-selection-retry'),
            onPressed: _interactionLocked ? null : _load,
            child: const Text('Retry'),
          ),
        if (_reloadRequired)
          TextButton(
            key: const Key('voice-status-reload'),
            onPressed: _busy ? null : _reloadTakes,
            child: const Text('Reload takes'),
          ),
        TextButton(
          key: const Key('voice-selection-cancel'),
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(
            _statusWasSaved || _requiresClose || _reloadRequired
                ? 'Close'
                : 'Cancel',
          ),
        ),
        FilledButton(
          key: const Key('voice-selection-save'),
          onPressed: _interactionLocked || !_hasChange ? null : _save,
          child: _busy
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Save selection'),
        ),
      ],
    );
  }
}

class _TakeChoiceTile extends StatelessWidget {
  const _TakeChoiceTile({
    required this.index,
    required this.take,
    required this.isCurrent,
    required this.busy,
    required this.statusDisabled,
    required this.statusBusy,
    required this.onStatusChanged,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;
  final bool statusDisabled;
  final bool statusBusy;
  final ValueChanged<AuthoringRevision3VoiceTakeStatus> onStatusChanged;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      RadioListTile<String>(
        key: ValueKey('voice-selection-take-$index'),
        value: take.id,
        enabled: !busy && take.isApproved,
        contentPadding: EdgeInsets.zero,
        title: Text(take.displayLabel),
        subtitle: Text(
          isCurrent && !take.isApproved
              ? '${take.statusLabel} • Current selection must be Approved; change to Approved or clear it'
              : '${take.statusLabel}${isCurrent ? ' • Current selection' : ''}${take.isApproved ? '' : ' • Approval required before selection'}',
        ),
      ),
      _TakeStatusControl(
        index: index,
        take: take,
        isCurrent: isCurrent,
        busy: busy,
        statusDisabled: statusDisabled,
        statusBusy: statusBusy,
        onStatusChanged: onStatusChanged,
      ),
    ],
  );
}

class _TakeStatusControl extends StatelessWidget {
  const _TakeStatusControl({
    required this.index,
    required this.take,
    required this.isCurrent,
    required this.busy,
    required this.statusDisabled,
    required this.statusBusy,
    required this.onStatusChanged,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;
  final bool statusDisabled;
  final bool statusBusy;
  final ValueChanged<AuthoringRevision3VoiceTakeStatus> onStatusChanged;

  @override
  Widget build(BuildContext context) {
    final currentStatus = AuthoringRevision3VoiceTakeStatus.values.byName(
      take.status.name,
    );
    final canChangeStatus = AuthoringRevision3VoiceTakeStatus.values.any(
      (status) =>
          status != currentStatus &&
          (!isCurrent || status == AuthoringRevision3VoiceTakeStatus.approved),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: PopupMenuButton<AuthoringRevision3VoiceTakeStatus>(
            key: ValueKey('voice-status-change-$index'),
            enabled: !busy && !statusDisabled && canChangeStatus,
            tooltip:
                isCurrent &&
                    currentStatus == AuthoringRevision3VoiceTakeStatus.approved
                ? 'Clear the selection before changing this status'
                : 'Change take status',
            onSelected: onStatusChanged,
            itemBuilder: (context) => [
              for (final status in AuthoringRevision3VoiceTakeStatus.values)
                PopupMenuItem<AuthoringRevision3VoiceTakeStatus>(
                  key: ValueKey('voice-status-option-$index-${status.name}'),
                  value: status,
                  enabled:
                      status != currentStatus &&
                      (!isCurrent ||
                          status == AuthoringRevision3VoiceTakeStatus.approved),
                  child: Text(_voiceTakeStatusLabel(status)),
                ),
            ],
            child: Padding(
              padding: const EdgeInsets.only(left: 12, bottom: 8),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (statusBusy) ...[
                    const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    const SizedBox(width: 8),
                  ],
                  Text(
                    'Change status...',
                    style: TextStyle(
                      color: !busy && !statusDisabled && canChangeStatus
                          ? Theme.of(context).colorScheme.primary
                          : Theme.of(context).disabledColor,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        if (isCurrent &&
            currentStatus == AuthoringRevision3VoiceTakeStatus.approved)
          const Padding(
            padding: EdgeInsets.only(left: 12, bottom: 8),
            child: Text(
              'Clear the selection before changing this take from Approved.',
            ),
          ),
      ],
    );
  }
}

String _voiceTakeStatusLabel(AuthoringRevision3VoiceTakeStatus status) =>
    switch (status) {
      AuthoringRevision3VoiceTakeStatus.draft => 'Draft',
      AuthoringRevision3VoiceTakeStatus.recorded => 'Recorded',
      AuthoringRevision3VoiceTakeStatus.reviewed => 'Reviewed',
      AuthoringRevision3VoiceTakeStatus.approved => 'Approved',
    };

bool _catalogConfirmsStatusPublication(
  Revision3VoiceCatalog catalog,
  Revision3VoiceTakeStatusPublication publication,
) {
  if (catalog.projectId != publication.projectId ||
      catalog.projectRevision != publication.projectRevision) {
    return false;
  }
  final line = catalog.line(publication.lineId);
  final summary = line?.slotSummaryForLocale(publication.locale);
  final take = summary?.candidate(publication.takeId);
  return line != null &&
      line.localizationId == publication.localizationId &&
      line.localizationIdentity == publication.locId &&
      line.slotIdForLocale(publication.locale) == publication.slotId &&
      summary != null &&
      summary.slotRevision == publication.slotRevision &&
      take != null &&
      take.revision == publication.takeRevision &&
      take.status.name == publication.status.name;
}

String _friendlyError(Object error) {
  if (error is ModFfiException) return error.message;
  if (error is FormatException) return error.message;
  return error.toString();
}
