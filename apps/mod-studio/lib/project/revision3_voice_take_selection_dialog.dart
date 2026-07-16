import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import '../l10n/app_localizations.dart';
import 'revision3_dialog_voice_slot_removal_authoring.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_removal_authoring.dart';
import 'revision3_voice_take_selection_authoring.dart';
import 'revision3_voice_take_status_authoring.dart';

const _clearSelectionValue = '__no_voice_take_selected__';
const _fixedContextUnavailableMessage =
    'This Voice action no longer matches one intact existing Voice setup in the exact current project. Close it and reopen Manage takes from the current workspace. No project, game, or save files were changed.';

/// Friendly project-only editor for selecting or clearing an existing Voice
/// take. Entity IDs, CAS heads, paths, and archive internals are never shown.
class Revision3VoiceTakeSelectionDialog extends StatefulWidget {
  const Revision3VoiceTakeSelectionDialog({
    super.key,
    required this.service,
    required this.statusService,
    required this.removalService,
    required this.slotRemovalService,
    this.initialLineId,
    this.initialLocale,
    this.fixedContext = false,
  });

  final Revision3VoiceTakeSelectionAuthoringService service;
  final Revision3VoiceTakeStatusAuthoringService statusService;
  final Revision3VoiceTakeRemovalAuthoringService removalService;
  final Revision3DialogVoiceSlotRemovalAuthoringService slotRemovalService;
  final String? initialLineId;
  final String? initialLocale;

  /// Keeps an in-workspace line/locale handoff fixed. The freshly loaded
  /// catalog must still prove that exact existing Voice setup intact.
  final bool fixedContext;

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
  String? _removalBusyTakeId;
  Revision3VoiceTakeStatusPublication? _pendingStatusPublication;
  Revision3VoiceTakeRemovalPublication? _pendingRemovalPublication;
  Revision3DialogVoiceSlotRemovalPublication? _pendingSlotRemovalPublication;
  bool _loading = true;
  bool _busy = false;
  bool _statusWasSaved = false;
  bool _removalWasSaved = false;
  bool _slotRemovalWasSaved = false;
  bool _requiresClose = false;
  bool _reloadRequired = false;
  bool _initialSelectionConsumed = false;
  bool _fixedContextInvalid = false;
  bool _catalogLoadFailed = false;
  int _loadGeneration = 0;

  bool get _interactionLocked =>
      _loading ||
      _catalogLoadFailed ||
      _catalog == null ||
      _busy ||
      _requiresClose ||
      _reloadRequired ||
      !_fixedContextIsCurrent;

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_searchChanged);
    _load();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTakeSelectionDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service) ||
        !identical(oldWidget.statusService, widget.statusService) ||
        !identical(oldWidget.removalService, widget.removalService) ||
        !identical(oldWidget.slotRemovalService, widget.slotRemovalService) ||
        oldWidget.fixedContext != widget.fixedContext ||
        oldWidget.initialLineId != widget.initialLineId ||
        oldWidget.initialLocale != widget.initialLocale) {
      _initialSelectionConsumed = false;
      _load(resetRecovery: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration++;
    _searchController
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _load({bool resetRecovery = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      if (resetRecovery) {
        if (_reloadRequired) _busy = false;
        _reloadRequired = false;
        _pendingStatusPublication = null;
        _pendingRemovalPublication = null;
        _pendingSlotRemovalPublication = null;
      }
      _loading = true;
      _catalogLoadFailed = false;
      _catalog = null;
      _lineId = null;
      _locale = null;
      _selectionValue = null;
      _fixedContextInvalid = false;
      _error = null;
      _notice = null;
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      Revision3VoiceDialogLineChoice? initialLine;
      String? initialLocale;
      var fixedContextInvalid = false;
      if (widget.fixedContext) {
        final requestedLine = catalog.line(widget.initialLineId ?? '');
        final requestedLocale = widget.initialLocale;
        if (requestedLine != null &&
            requestedLocale != null &&
            _intactLocalesFor(requestedLine).contains(requestedLocale)) {
          initialLine = requestedLine;
          initialLocale = requestedLocale;
        } else {
          fixedContextInvalid = true;
        }
      } else {
        initialLine = _initialSelectionConsumed
            ? null
            : catalog.line(widget.initialLineId ?? '');
        final initialLocales = initialLine == null
            ? const <String>[]
            : _intactLocalesFor(initialLine);
        initialLocale = initialLocales.contains(widget.initialLocale)
            ? widget.initialLocale
            : initialLocales.firstOrNull;
        _initialSelectionConsumed = true;
      }
      setState(() {
        _catalog = catalog;
        _lineId = initialLine?.lineId;
        _locale = initialLocale;
        _selectionValue = initialLine == null || initialLocale == null
            ? null
            : _selectionValueFor(
                initialLine.slotSummaryForLocale(initialLocale)!,
              );
        _fixedContextInvalid = fixedContextInvalid;
        _catalogLoadFailed = false;
        if (fixedContextInvalid) {
          _error = _fixedContextUnavailableMessage;
        }
        _loading = false;
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalogLoadFailed = true;
        _requiresClose = true;
        _error = 'Reopen the managed project before changing Voice takes.';
      });
    } catch (error) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalogLoadFailed = true;
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

  static List<String> _intactLocalesFor(Revision3VoiceDialogLineChoice line) =>
      line.existingSlotLocales
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

  bool get _fixedContextIsCurrent {
    if (!widget.fixedContext) return true;
    final line = _selectedLine;
    final locale = _locale;
    return !_fixedContextInvalid &&
        line != null &&
        line.lineId == widget.initialLineId &&
        locale != null &&
        locale == widget.initialLocale &&
        line.slotSummaryForLocale(locale) != null &&
        _intactLocalesFor(line).contains(locale);
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

  Future<void> _confirmRemove(Revision3VoiceCandidateTake take) async {
    final line = _selectedLine;
    final locale = _locale;
    final summary = _selectedSummary;
    if (_interactionLocked ||
        _hasChange ||
        line == null ||
        locale == null ||
        summary == null ||
        summary.candidate(take.id) == null) {
      return;
    }
    final selected = summary.selectedTakeId == take.id;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) {
        final copy = AppLocalizations.of(dialogContext);
        return AlertDialog(
          key: const Key('voice-take-remove-confirm-dialog'),
          title: Text(copy.managedVoiceTakeRemoveDialogTitle),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    copy.managedVoiceTakeRemoveDialogSummary(
                      take.displayLabel,
                      line.displayLabel,
                      locale,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Text(copy.managedVoiceTakeRemoveScope),
                  const SizedBox(height: 8),
                  Text(copy.managedVoiceTakeRemoveInternalRetention),
                  const SizedBox(height: 8),
                  Text(copy.managedVoiceTakeRemoveGameBoundary),
                  if (selected) ...[
                    const SizedBox(height: 12),
                    Container(
                      key: const Key('voice-take-remove-selected-warning'),
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Theme.of(
                          dialogContext,
                        ).colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Icon(
                            Icons.warning_amber_rounded,
                            color: Theme.of(
                              dialogContext,
                            ).colorScheme.onErrorContainer,
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Text(
                              copy.managedVoiceTakeRemoveSelectedWarning,
                              style: TextStyle(
                                color: Theme.of(
                                  dialogContext,
                                ).colorScheme.onErrorContainer,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              key: const Key('voice-take-remove-cancel'),
              onPressed: () => Navigator.of(dialogContext).pop(false),
              child: Text(copy.managedVoiceTakeRemoveCancel),
            ),
            FilledButton(
              key: const Key('voice-take-remove-confirm'),
              onPressed: () => Navigator.of(dialogContext).pop(true),
              child: Text(copy.managedVoiceTakeRemoveConfirm),
            ),
          ],
        );
      },
    );
    if (confirmed != true || !mounted) return;
    await _removeTake(take);
  }

  Future<void> _removeTake(Revision3VoiceCandidateTake take) async {
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
    Revision3VoiceTakeRemovalPublication? publication;
    setState(() {
      _busy = true;
      _removalBusyTakeId = take.id;
      _error = null;
      _notice = null;
    });
    try {
      publication = await widget.removalService.publish(
        checkpoint: catalog,
        lineId: lineId,
        locale: locale,
        takeId: take.id,
      );
      publishedThisAttempt = true;
      _removalWasSaved = true;
      _pendingRemovalPublication = publication;
      final refreshed = await widget.removalService.loadCatalog();
      if (!_catalogConfirmsRemoval(refreshed, publication)) {
        throw const Revision3VoiceTakeRemovalRequiresReopenException();
      }
      final refreshedLine = refreshed.line(lineId)!;
      final refreshedSummary = refreshedLine.slotSummaryForLocale(locale)!;
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _catalog = refreshed;
        _lineId = refreshedLine.lineId;
        _locale = locale;
        _selectionValue = _selectionValueFor(refreshedSummary);
        _busy = false;
        _removalBusyTakeId = null;
        _pendingRemovalPublication = null;
        final outcome = publication!.takeEntityRemoved
            ? copy.managedVoiceTakeRemoveUniqueSuccess
            : copy.managedVoiceTakeRemoveSharedSuccess;
        _notice = publication.selectionCleared
            ? '$outcome\n${copy.managedVoiceTakeRemoveSelectionClearedSuccess}'
            : outcome;
      });
    } on Revision3VoiceTakeRemovalStaleCheckpointException {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = true;
        _error = copy.managedVoiceTakeRemoveStale;
      });
    } on Revision3VoiceTakeRemovalRequiresReopenException {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = false;
        _requiresClose = true;
        _error = publishedThisAttempt
            ? copy.managedVoiceTakeRemoveSavedUnconfirmed
            : copy.managedVoiceTakeRemoveRequiresReopen;
      });
    } catch (error) {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? copy.managedVoiceTakeRemoveSavedReloadFailed
            : copy.managedVoiceTakeRemoveFailed(_friendlyError(error));
      });
    }
  }

  Future<void> _confirmRemoveEmptySlot() async {
    final line = _selectedLine;
    final locale = _locale;
    final summary = _selectedSummary;
    if (_interactionLocked ||
        _hasChange ||
        line == null ||
        locale == null ||
        summary == null ||
        !summary.isRemovableGeneratedSlot ||
        summary.candidateCount != 0 ||
        summary.selectedTakeId != null) {
      return;
    }
    final retainsTargetEvidence = summary.targetResolution.name != 'unresolved';
    final copy = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('voice-slot-remove-confirm-dialog'),
        title: Text(copy.managedVoiceSlotRemoveDialogTitle),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  copy.managedVoiceSlotRemoveDialogSummary(
                    line.displayLabel,
                    locale,
                  ),
                ),
                const SizedBox(height: 12),
                Text(copy.managedVoiceSlotRemoveRetention),
                if (retainsTargetEvidence) ...[
                  const SizedBox(height: 12),
                  Container(
                    key: const Key('voice-slot-remove-target-warning'),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Theme.of(dialogContext).colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(copy.managedVoiceSlotRemoveTargetWarning),
                  ),
                ],
                const SizedBox(height: 12),
                Text(copy.managedVoiceSlotRemoveRecreate),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('voice-slot-remove-cancel'),
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(copy.managedVoiceSlotRemoveCancel),
          ),
          FilledButton(
            key: const Key('voice-slot-remove-confirm'),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(copy.managedVoiceSlotRemoveConfirm),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    await _removeEmptySlot();
  }

  Future<void> _removeEmptySlot() async {
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
    Revision3DialogVoiceSlotRemovalPublication? publication;
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      publication = await widget.slotRemovalService.publish(
        checkpoint: catalog,
        lineId: lineId,
        locale: locale,
      );
      publishedThisAttempt = true;
      _slotRemovalWasSaved = true;
      _pendingSlotRemovalPublication = publication;
      final refreshed = await widget.slotRemovalService.loadCatalog();
      if (!_catalogConfirmsSlotRemoval(refreshed, publication)) {
        throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
      }
      final refreshedLine = refreshed.line(lineId);
      final locales = refreshedLine == null
          ? const <String>[]
          : _intactLocales(refreshedLine);
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _catalog = refreshed;
        if (widget.fixedContext) {
          _lineId = refreshedLine?.lineId;
          _locale = locale;
          _selectionValue = null;
          _requiresClose = true;
        } else {
          _lineId = locales.isEmpty ? null : refreshedLine!.lineId;
          _locale = locales.firstOrNull;
          _selectionValue = _locale == null
              ? null
              : _selectionValueFor(
                  refreshedLine!.slotSummaryForLocale(_locale!)!,
                );
        }
        _busy = false;
        _pendingSlotRemovalPublication = null;
        _notice = copy.managedVoiceSlotRemoveSuccess;
      });
    } on Revision3DialogVoiceSlotRemovalStaleCheckpointException {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _reloadRequired = true;
        _error = copy.managedVoiceSlotRemoveStale;
      });
    } on Revision3DialogVoiceSlotRemovalRequiresReopenException {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = copy.managedVoiceSlotRemoveSavedUnconfirmed;
      });
    } catch (error) {
      if (!mounted) return;
      final copy = AppLocalizations.of(context);
      setState(() {
        _busy = false;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? copy.managedVoiceSlotRemoveSavedReloadFailed
            : copy.managedVoiceSlotRemoveFailed(_friendlyError(error));
      });
    }
  }

  Future<void> _reloadTakes() async {
    if (_busy || !_reloadRequired || _requiresClose) return;
    final generation = ++_loadGeneration;
    final service = widget.service;
    final statusService = widget.statusService;
    final removalService = widget.removalService;
    final slotRemovalService = widget.slotRemovalService;
    final fixedContext = widget.fixedContext;
    final initialLineId = widget.initialLineId;
    final initialLocale = widget.initialLocale;
    bool recoveryIsCurrent() =>
        mounted &&
        generation == _loadGeneration &&
        identical(widget.service, service) &&
        identical(widget.statusService, statusService) &&
        identical(widget.removalService, removalService) &&
        identical(widget.slotRemovalService, slotRemovalService) &&
        widget.fixedContext == fixedContext &&
        widget.initialLineId == initialLineId &&
        widget.initialLocale == initialLocale;
    final previous = _catalog;
    final pending = _pendingStatusPublication;
    final pendingRemoval = _pendingRemovalPublication;
    final pendingSlotRemoval = _pendingSlotRemovalPublication;
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      final refreshed = await statusService.loadCatalog();
      if (!recoveryIsCurrent()) return;
      if (previous == null || refreshed.projectId != previous.projectId) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      if (pending != null &&
          !_catalogConfirmsStatusPublication(refreshed, pending)) {
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      if (pendingRemoval != null &&
          !_catalogConfirmsRemoval(refreshed, pendingRemoval)) {
        throw const Revision3VoiceTakeRemovalRequiresReopenException();
      }
      if (pendingSlotRemoval != null &&
          !_catalogConfirmsSlotRemoval(refreshed, pendingSlotRemoval)) {
        throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
      }
      Revision3VoiceDialogLineChoice? line;
      String? locale;
      Revision3VoiceExistingSlotSummary? summary;
      if (fixedContext) {
        final requestedLocale = initialLocale;
        line = refreshed.line(initialLineId ?? '');
        locale = requestedLocale;
        summary = line == null || requestedLocale == null
            ? null
            : line.slotSummaryForLocale(requestedLocale);
        final exactContextIsCurrent =
            line != null &&
            requestedLocale != null &&
            summary != null &&
            _intactLocalesFor(line).contains(requestedLocale);
        final confirmedRequestedSlotRemoval = pendingSlotRemoval != null;
        if (!exactContextIsCurrent && !confirmedRequestedSlotRemoval) {
          if (!recoveryIsCurrent()) return;
          setState(() {
            _catalog = refreshed;
            _lineId = null;
            _locale = null;
            _selectionValue = null;
            _pendingStatusPublication = null;
            _pendingRemovalPublication = null;
            _pendingSlotRemovalPublication = null;
            _reloadRequired = false;
            _busy = false;
            _fixedContextInvalid = true;
            _requiresClose = true;
            _error = _fixedContextUnavailableMessage;
            _notice = null;
          });
          return;
        }
      } else {
        line = _lineId == null ? null : refreshed.line(_lineId!);
        final locales = line == null ? const <String>[] : _intactLocales(line);
        locale = _locale != null && locales.contains(_locale)
            ? _locale
            : locales.firstOrNull;
        summary = locale == null ? null : line!.slotSummaryForLocale(locale);
      }
      if (!recoveryIsCurrent()) return;
      setState(() {
        _catalog = refreshed;
        _lineId = locale == null ? null : line!.lineId;
        _locale = locale;
        _selectionValue = summary == null ? null : _selectionValueFor(summary);
        _pendingStatusPublication = null;
        _pendingRemovalPublication = null;
        _pendingSlotRemovalPublication = null;
        _reloadRequired = false;
        _busy = false;
        if (fixedContext && pendingSlotRemoval != null) {
          _requiresClose = true;
        }
        _notice = pendingSlotRemoval != null
            ? AppLocalizations.of(context).managedVoiceSlotRemoveReloadConfirmed
            : pendingRemoval != null
            ? AppLocalizations.of(context).managedVoiceTakeRemoveReloadConfirmed
            : pending == null
            ? 'Latest Voice takes reloaded.'
            : 'Saved status confirmed from the latest project.';
      });
    } on Revision3VoiceTakeRemovalRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = AppLocalizations.of(
          context,
        ).managedVoiceTakeRemoveSavedUnconfirmed;
      });
    } on Revision3DialogVoiceSlotRemovalRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = AppLocalizations.of(
          context,
        ).managedVoiceSlotRemoveSavedUnconfirmed;
      });
    } on Revision3VoiceTakeStatusRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error =
            'The latest Voice takes could not be confirmed. Close this window and reopen the managed project.';
      });
    } catch (error) {
      if (!recoveryIsCurrent()) return;
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
    final copy = AppLocalizations.of(context);
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
              if (widget.fixedContext && line != null && _locale != null) ...[
                _SelectionFixedContextBreadcrumb(
                  lineLabel: line.displayLabel,
                  locale: _locale!,
                ),
                const SizedBox(height: 8),
              ],
              if (!widget.fixedContext) ...[
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
              ],
              if (_loading)
                const Center(
                  child: Padding(
                    padding: EdgeInsets.all(24),
                    child: CircularProgressIndicator(),
                  ),
                )
              else if (!widget.fixedContext &&
                  _catalog != null &&
                  lines.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 16),
                  child: Text(
                    'No matching dialog line has an intact existing Voice slot.',
                    key: Key('voice-selection-no-lines'),
                  ),
                )
              else if (!widget.fixedContext && _catalog != null)
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
              if (!widget.fixedContext && line != null) ...[
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
                          removeLabel: copy.managedVoiceTakeRemoveAction,
                          removeTooltip: copy.managedVoiceTakeRemoveTooltip,
                          removeBusy:
                              _removalBusyTakeId ==
                              summary.candidates[index].id,
                          onRemove: () =>
                              _confirmRemove(summary.candidates[index]),
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
                  Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Text(
                          'This Voice setup has no takes.',
                          key: Key('voice-selection-no-candidates'),
                        ),
                        if (summary.isRemovableGeneratedSlot) ...[
                          const SizedBox(height: 8),
                          OutlinedButton.icon(
                            key: const Key('voice-slot-remove-empty'),
                            onPressed: _interactionLocked || _hasChange
                                ? null
                                : _confirmRemoveEmptySlot,
                            icon: const Icon(Icons.link_off),
                            label: Text(copy.managedVoiceSlotRemoveAction),
                          ),
                        ],
                      ],
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
        if (_error != null &&
            (_catalog == null || _fixedContextInvalid) &&
            !_loading &&
            !_requiresClose)
          TextButton(
            key: const Key('voice-selection-retry'),
            onPressed: _busy ? null : _load,
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
            _statusWasSaved ||
                    _removalWasSaved ||
                    _slotRemovalWasSaved ||
                    _requiresClose ||
                    _reloadRequired
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

class _SelectionFixedContextBreadcrumb extends StatelessWidget {
  const _SelectionFixedContextBreadcrumb({
    required this.lineLabel,
    required this.locale,
  });

  final String lineLabel;
  final String locale;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('voice-selection-fixed-context'),
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

class _TakeChoiceTile extends StatelessWidget {
  const _TakeChoiceTile({
    required this.index,
    required this.take,
    required this.isCurrent,
    required this.busy,
    required this.statusDisabled,
    required this.statusBusy,
    required this.onStatusChanged,
    required this.removeLabel,
    required this.removeTooltip,
    required this.removeBusy,
    required this.onRemove,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;
  final bool statusDisabled;
  final bool statusBusy;
  final ValueChanged<AuthoringRevision3VoiceTakeStatus> onStatusChanged;
  final String removeLabel;
  final String removeTooltip;
  final bool removeBusy;
  final VoidCallback onRemove;

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
        removeLabel: removeLabel,
        removeTooltip: removeTooltip,
        removeBusy: removeBusy,
        onRemove: onRemove,
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
    required this.removeLabel,
    required this.removeTooltip,
    required this.removeBusy,
    required this.onRemove,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;
  final bool statusDisabled;
  final bool statusBusy;
  final ValueChanged<AuthoringRevision3VoiceTakeStatus> onStatusChanged;
  final String removeLabel;
  final String removeTooltip;
  final bool removeBusy;
  final VoidCallback onRemove;

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
        Padding(
          padding: const EdgeInsets.only(left: 4, right: 4, bottom: 8),
          child: Wrap(
            spacing: 4,
            runSpacing: 4,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              PopupMenuButton<AuthoringRevision3VoiceTakeStatus>(
                key: ValueKey('voice-status-change-$index'),
                enabled: !busy && !statusDisabled && canChangeStatus,
                tooltip:
                    isCurrent &&
                        currentStatus ==
                            AuthoringRevision3VoiceTakeStatus.approved
                    ? 'Clear the selection before changing this status'
                    : 'Change take status',
                onSelected: onStatusChanged,
                itemBuilder: (context) => [
                  for (final status in AuthoringRevision3VoiceTakeStatus.values)
                    PopupMenuItem<AuthoringRevision3VoiceTakeStatus>(
                      key: ValueKey(
                        'voice-status-option-$index-${status.name}',
                      ),
                      value: status,
                      enabled:
                          status != currentStatus &&
                          (!isCurrent ||
                              status ==
                                  AuthoringRevision3VoiceTakeStatus.approved),
                      child: Text(_voiceTakeStatusLabel(status)),
                    ),
                ],
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 8,
                  ),
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
              Tooltip(
                message: removeTooltip,
                child: TextButton.icon(
                  key: ValueKey('voice-take-remove-$index'),
                  onPressed: busy || statusDisabled ? null : onRemove,
                  icon: removeBusy
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.link_off_outlined, size: 18),
                  label: Text(removeLabel),
                ),
              ),
            ],
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

bool _catalogConfirmsRemoval(
  Revision3VoiceCatalog catalog,
  Revision3VoiceTakeRemovalPublication publication,
) {
  if (catalog.projectId != publication.projectId ||
      catalog.projectRevision != publication.projectRevision) {
    return false;
  }
  final line = catalog.line(publication.lineId);
  final summary = line?.slotSummaryForLocale(publication.locale);
  if (line == null ||
      line.localizationId != publication.localizationId ||
      line.localizationIdentity != publication.locId ||
      line.slotIdForLocale(publication.locale) != publication.slotId ||
      summary == null ||
      summary.slotRevision != publication.slotRevision ||
      summary.candidate(publication.takeId) != null ||
      summary.candidateCount != publication.remainingCandidateCount ||
      summary.selectedTakeId !=
          (publication.selectionCleared
              ? null
              : publication.previousSelectedTakeId)) {
    return false;
  }
  final takeStillExists = catalog.entityIds.contains(publication.takeId);
  return publication.takeEntityRemoved != takeStillExists;
}

bool _catalogConfirmsSlotRemoval(
  Revision3VoiceCatalog catalog,
  Revision3DialogVoiceSlotRemovalPublication publication,
) {
  if (catalog.projectId != publication.projectId ||
      catalog.projectRevision != publication.projectRevision ||
      catalog.entityIds.contains(publication.slotId)) {
    return false;
  }
  final line = catalog.line(publication.lineId);
  return line != null &&
      line.lineRevision == publication.lineRevision &&
      line.localizationId == publication.localizationId &&
      line.localizationIdentity == publication.locId &&
      line.slotIdForLocale(publication.locale) == null &&
      line.slotSummaryForLocale(publication.locale) == null;
}

String _friendlyError(Object error) {
  if (error is ModFfiException) return error.message;
  if (error is FormatException) return error.message;
  return error.toString();
}
