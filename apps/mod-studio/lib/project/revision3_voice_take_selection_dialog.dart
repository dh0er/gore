import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_dialog_voice_slot_removal_authoring.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_removal_authoring.dart';
import 'revision3_voice_take_preview_playback.dart';
import 'revision3_voice_take_selection_authoring.dart';
import 'revision3_voice_take_status_authoring.dart';

const _clearSelectionValue = '__no_voice_take_selected__';

typedef Revision3VoiceTakePreviewDialogMaterializer =
    Future<Revision3VoiceTakePreviewPlaybackLease> Function({
      required Revision3VoiceCatalog checkpoint,
      required String lineId,
      required String locale,
      required String takeId,
    });

/// Complete author-facing copy for [Revision3VoiceTakeSelectionDialog].
@immutable
final class Revision3VoiceTakeSelectionDialogCopy {
  const Revision3VoiceTakeSelectionDialogCopy._(this._german);

  static const english = Revision3VoiceTakeSelectionDialogCopy._(false);
  static const german = Revision3VoiceTakeSelectionDialogCopy._(true);

  final bool _german;

  String get title => _german ? 'Voice-Takes verwalten' : 'Manage Voice takes';
  String get introduction => _german
      ? 'Wähle, welche vorhandene freigegebene Aufnahme diese Dialogzeile verwenden soll. Der Status ist nur eine Kennzeichnung im Autoren-Workflow und belegt weder Audioqualität noch Einsatzbereitschaft im Spiel. Änderungen bleiben im Offline-Projekt, bis Build und Bereitstellung separat ausgeführt werden.'
      : 'Choose which existing Approved recording this dialog line should use. Status is an author workflow label only; it does not prove audio quality or in-game readiness. Changes stay in the offline project until separate build and deployment steps.';
  String get fixedContextUnavailable => _german
      ? 'Diese Voice-Aktion passt im exakt aktuellen Projekt nicht mehr zu einem intakten vorhandenen Voice-Setup. Schließe das Fenster und öffne „Aufnahmen verwalten“ im aktuellen Arbeitsbereich erneut. Es wurden keine Projekt-, Spiel- oder Spielstanddateien verändert.'
      : 'This Voice action no longer matches one intact existing Voice setup in the exact current project. Close it and reopen Manage takes from the current workspace. No project, game, or save files were changed.';
  String get requiresReopen => _german
      ? 'Öffne das verwaltete Projekt erneut, bevor du Voice-Takes änderst.'
      : 'Reopen the managed project before changing Voice takes.';
  String get loadFailed => _german
      ? 'Voice-Takes konnten nicht sicher geladen werden. Versuche es erneut oder öffne das verwaltete Projekt neu.'
      : 'Voice takes could not be loaded safely. Try again or reopen the managed project.';
  String get staleSelection => _german
      ? 'Das Projekt wurde geändert, während dieses Fenster geöffnet war. Schließe es und versuche es im aktualisierten Projekt erneut.'
      : 'The project changed while this window was open. Close it and try again from the refreshed project.';
  String get selectionSaveFailed => _german
      ? 'Die Voice-Auswahl konnte nicht sicher gespeichert werden. Es wurden keine Spiel- oder Spielstanddateien geändert.'
      : 'The Voice selection could not be saved safely. No game or save files were changed.';

  String get statusChangedApproved => _german
      ? 'Status wurde auf Freigegeben geändert. Dieser Take kann jetzt ausgewählt werden.'
      : 'Status changed to Approved. This take can now be selected.';
  String statusChanged(String status) => _german
      ? 'Status wurde auf $status geändert.'
      : 'Status changed to $status.';
  String get selectedTakeStatusBlocked => _german
      ? 'Dieser Take ist aktuell ausgewählt. Leere die Auswahl, bevor du seinen Status von Freigegeben änderst.'
      : 'This take is currently selected. Clear the selection before changing it from Approved.';
  String get statusStale => _german
      ? 'Das Projekt wurde geändert, bevor dieser Status gespeichert werden konnte. Lade die aktuellen Voice-Takes neu, bevor du fortfährst.'
      : 'The project changed before this status could be saved. Reload the latest Voice takes before continuing.';
  String get statusSavedUnconfirmed => _german
      ? 'Der Status wurde gespeichert, aber die aktuellen Voice-Takes konnten nicht bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut.'
      : 'The status was saved, but the latest Voice takes could not be confirmed. Close this window and reopen the managed project.';
  String get statusUnconfirmed => _german
      ? 'Das Statusergebnis konnte nicht bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut, bevor du es erneut versuchst.'
      : 'The status result could not be confirmed. Close this window and reopen the managed project before trying again.';
  String get statusSavedReloadFailed => _german
      ? 'Der Status wurde gespeichert, aber die aktuellen Voice-Takes konnten nicht bestätigt werden. Lade die Takes neu, bevor du fortfährst; die Statusänderung wird nicht wiederholt.'
      : 'The status was saved, but the latest Voice takes could not be confirmed. Reload the takes before continuing; the status change will not be repeated.';
  String get statusSaveFailed => _german
      ? 'Der Voice-Take-Status konnte nicht sicher gespeichert werden. Prüfe den aktuellen Projektstand und versuche es erneut.'
      : 'The Voice take status could not be saved safely. Review the current project and try again.';

  String get takeRemoveAction =>
      _german ? 'Aus dieser Zeile entfernen…' : 'Remove from this line…';
  String get takeRemoveTooltip => _german
      ? 'Diese Aufnahme aus der aktuellen Dialogzeile und Sprache entfernen'
      : 'Remove this recording from the current dialog line and language';
  String get takeRemoveDialogTitle =>
      _german ? 'Voice-Take entfernen?' : 'Remove Voice take?';
  String takeRemoveDialogSummary(String take, String line, String locale) =>
      _german
      ? '„$take“ aus $line ($locale) entfernen?'
      : 'Remove “$take” from $line ($locale)?';
  String get takeRemoveScope => _german
      ? 'Nur die Verknüpfung für diese Dialogzeile und Sprache wird gelöst. Andere Verwendungen im Projekt bleiben unverändert.'
      : 'Only the link for this dialog line and language is removed. Other project uses remain unchanged.';
  String get takeRemoveInternalRetention => _german
      ? 'Die Audiodatei bleibt intern gespeichert. Diese Aktion gibt keinen Projektspeicher frei und kann noch nicht rückgängig gemacht werden.'
      : 'The audio file remains stored internally. This action does not free project storage and has no undo yet.';
  String get takeRemoveGameBoundary => _german
      ? 'Spielinstallation und Spielstände werden nicht verändert.'
      : 'The game installation and save games are not changed.';
  String get takeRemoveSelectedWarning => _german
      ? 'Dies ist der aktive Take. Beim Entfernen wird die Auswahl atomar geleert. Es wird kein Ersatz automatisch gewählt; der Voice-Build bleibt blockiert, bis ein freigegebener Take ausgewählt wurde.'
      : 'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.';
  String get takeRemoveCancel => _german ? 'Abbrechen' : 'Cancel';
  String get takeRemoveConfirm =>
      _german ? 'Aus Zeile entfernen' : 'Remove from line';
  String get takeRemoveUniqueSuccess => _german
      ? 'Der Take wurde aus dieser Zeile und dem aktuellen Projektgraphen entfernt. Seine internen Audiodaten bleiben erhalten.'
      : 'The take was removed from this line and from the current project graph. Its internal audio data remains retained.';
  String get takeRemoveSharedSuccess => _german
      ? 'Die Verknüpfung wurde aus dieser Zeile und Sprache gelöst. Der Take bleibt für andere Verwendungen im Projekt verfügbar; seine internen Audiodaten bleiben erhalten.'
      : 'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.';
  String get takeRemoveSelectionClearedSuccess => _german
      ? 'Die aktive Auswahl wurde atomar geleert. Es wurde kein Ersatz gewählt; der Voice-Build bleibt blockiert, bis ein freigegebener Take ausgewählt wurde.'
      : 'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.';
  String get takeRemoveStale => _german
      ? 'Das Projekt wurde geändert, bevor der Take entfernt werden konnte. Lade die aktuellen Voice-Takes neu und prüfe die Aktion erneut.'
      : 'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.';
  String get takeRemoveRequiresReopen => _german
      ? 'Das Ergebnis der Entfernung konnte nicht bestätigt werden. Nicht erneut versuchen. Schließe dieses Fenster und öffne das verwaltete Projekt erneut oder stelle es wieder her.'
      : 'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.';
  String get takeRemoveSavedUnconfirmed => _german
      ? 'Die Entfernung wurde gespeichert, aber der aktuelle Projektstand konnte nicht bestätigt werden. Wiederhole die Entfernung nicht. Schließe dieses Fenster und öffne das verwaltete Projekt erneut oder stelle es wieder her.'
      : 'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.';
  String get takeRemoveSavedReloadFailed => _german
      ? 'Die Entfernung wurde gespeichert, aber die aktuellen Voice-Takes konnten nicht geladen werden. Lade die Takes neu; die Entfernung wird nicht wiederholt.'
      : 'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.';
  String get takeRemoveFailed => _german
      ? 'Der Take konnte nicht sicher entfernt werden. Es wurde keine Entfernung bestätigt.'
      : 'The take could not be removed safely. No removal was confirmed.';
  String get takeRemoveReloadConfirmed => _german
      ? 'Die gespeicherte Entfernung wurde im aktuellen Projektstand bestätigt.'
      : 'The saved removal was confirmed from the latest project.';

  String get slotRemoveAction =>
      _german ? 'Leeres Voice-Setup entfernen…' : 'Remove empty Voice setup…';
  String get slotRemoveDialogTitle =>
      _german ? 'Leeres Voice-Setup entfernen?' : 'Remove empty Voice setup?';
  String slotRemoveDialogSummary(String line, String locale) => _german
      ? 'Das leere Voice-Setup für $locale aus $line entfernen?'
      : 'Remove the empty $locale Voice setup from $line?';
  String get slotRemoveRetention => _german
      ? 'Der Dialogtext bleibt im Projekt. Keine Aufnahme, kein Audio-Blob, keine Spieldatei und kein Spielstand werden gelöscht.'
      : 'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.';
  String get slotRemoveTargetWarning => _german
      ? 'Dabei wird auch der gespeicherte Nachweis zum installierten Ziel für diese Zeile und Sprache entfernt. Das installierte Archiv selbst bleibt unberührt.'
      : 'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.';
  String get slotRemoveRecreate => _german
      ? 'Du kannst später einen neuen Take hinzufügen; das benötigte Voice-Setup wird dann automatisch neu erstellt.'
      : 'You can add a new take later; the required Voice setup will then be created again automatically.';
  String get slotRemoveCancel => _german ? 'Setup behalten' : 'Keep setup';
  String get slotRemoveConfirm => _german ? 'Setup entfernen' : 'Remove setup';
  String get slotRemoveSuccess => _german
      ? 'Das leere Voice-Setup wurde entfernt. Dialogtext, Audiospeicher, Spieldateien und Spielstände wurden nicht verändert.'
      : 'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.';
  String get slotRemoveStale => _german
      ? 'Das Projekt wurde geändert, bevor das leere Voice-Setup entfernt werden konnte. Lade die aktuellen Voice-Takes neu und versuche es erneut.'
      : 'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.';
  String get slotRemoveSavedUnconfirmed => _german
      ? 'Das Ergebnis konnte nicht bestätigt werden; das leere Voice-Setup wurde möglicherweise gespeichert. Wiederhole die Entfernung nicht. Schließe dieses Fenster, öffne das verwaltete Projekt erneut und prüfe die Zeile.'
      : 'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.';
  String get slotRemoveSavedReloadFailed => _german
      ? 'Das leere Voice-Setup wurde gespeichert, aber das Neuladen ist fehlgeschlagen. Lade neu, um die Änderung zu bestätigen; die Entfernung wird nicht wiederholt.'
      : 'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.';
  String get slotRemoveFailed => _german
      ? 'Das leere Voice-Setup konnte nicht sicher entfernt werden. Es wurde keine Entfernung bestätigt.'
      : 'The empty Voice setup could not be removed safely. No removal was confirmed.';
  String get slotRemoveReloadConfirmed => _german
      ? 'Die gespeicherte Entfernung des leeren Voice-Setups wurde im aktuellen Projektstand bestätigt.'
      : 'Saved empty Voice setup removal confirmed from the latest project.';

  String get latestTakesReloaded => _german
      ? 'Aktuelle Voice-Takes neu geladen.'
      : 'Latest Voice takes reloaded.';
  String get savedStatusConfirmed => _german
      ? 'Gespeicherter Status im aktuellen Projekt bestätigt.'
      : 'Saved status confirmed from the latest project.';
  String get latestTakesUnconfirmed => _german
      ? 'Die aktuellen Voice-Takes konnten nicht bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut.'
      : 'The latest Voice takes could not be confirmed. Close this window and reopen the managed project.';
  String get reloadFailed => _german
      ? 'Voice-Takes konnten nicht sicher neu geladen werden. Keine gespeicherte Änderung wurde wiederholt.'
      : 'Voice takes could not be reloaded safely. No saved change was repeated.';

  String get findLineLabel =>
      _german ? 'Dialogzeile finden' : 'Find a dialog line';
  String get findLineHint => _german
      ? 'Nach Sprecher oder Zeilenname suchen'
      : 'Search by speaker or line name';
  String get noMatchingLine => _german
      ? 'Keine passende Dialogzeile besitzt ein intaktes vorhandenes Voice-Setup.'
      : 'No matching dialog line has an intact existing Voice slot.';
  String voiceLanguageCount(int count) => _german
      ? '$count Voice-Sprache${count == 1 ? '' : 'n'}'
      : '$count Voice language${count == 1 ? '' : 's'}';
  String get voiceLanguageLabel => _german ? 'Voice-Sprache' : 'Voice language';
  String get selectedTakeTitle =>
      _german ? 'Ausgewählter Take' : 'Selected take';
  String get noTakeSelected =>
      _german ? 'Kein Take ausgewählt' : 'No take selected';
  String get currentSelection =>
      _german ? 'Aktuelle Auswahl' : 'Current selection';
  String get clearActiveChoice => _german
      ? 'Aufnahmen behalten, aber aktive Auswahl leeren'
      : 'Keep the recordings, but clear the active choice';
  String get clearSelectionWarning => _german
      ? 'Die Takes bleiben in diesem Projekt, aber der Voice-Build ist blockiert, bis wieder ein freigegebener Take ausgewählt wurde.'
      : 'The takes stay in this project, but Voice build is blocked until an Approved take is selected again.';
  String get pendingSelectionStatus => _german
      ? 'Speichere die ausstehende Auswahl oder mache sie rückgängig, bevor du einen Take-Status änderst.'
      : 'Save or undo the pending selection before changing a take status.';
  String get noTakes => _german
      ? 'Dieses Voice-Setup hat keine Takes.'
      : 'This Voice setup has no takes.';
  String get previewAction => _german ? 'Anhören' : 'Preview';
  String get previewPause => _german ? 'Pause' : 'Pause';
  String get previewResume => _german ? 'Fortsetzen' : 'Resume';
  String get previewReplay => _german ? 'Erneut anhören' : 'Replay';
  String get previewStop => _german ? 'Vorschau stoppen' : 'Stop preview';
  String get previewPreparing => _german
      ? 'Sichere Vorschau wird vorbereitet…'
      : 'Preparing secure preview…';
  String get previewUnavailable => _german
      ? 'Für diesen Take ist keine intakte Vorschau verfügbar.'
      : 'No intact preview is available for this take.';
  String get previewFailed => _german
      ? 'Die Vorschau konnte nicht im Mod Studio abgespielt werden.'
      : 'The preview could not be played inside Mod Studio.';
  String get previewStale => _german
      ? 'Das Projekt wurde seit dem Laden dieser Takes geändert. Lade die aktuellen Takes neu.'
      : 'The project changed since these takes were loaded. Reload the latest takes.';
  String get previewRequiresReopen => _german
      ? 'Öffne das verwaltete Projekt erneut, bevor du weitere Takes anhörst.'
      : 'Reopen the managed project before previewing more takes.';
  String get previewCleanupFailed => _german
      ? 'Die vorige Vorschau konnte nicht sicher geschlossen werden. Stoppe sie und versuche die sichere Bereinigung erneut.'
      : 'The previous preview could not be closed safely. Stop it and retry the safe cleanup.';
  String get retry => _german ? 'Erneut versuchen' : 'Retry';
  String get reloadTakes => _german ? 'Takes neu laden' : 'Reload takes';
  String get close => _german ? 'Schließen' : 'Close';
  String get cancel => _german ? 'Abbrechen' : 'Cancel';
  String get saveSelection => _german ? 'Auswahl speichern' : 'Save selection';
  String fixedContextLanguage(String locale) =>
      _german ? 'Voice-Sprache: $locale' : 'Voice language: $locale';

  String statusLabel(String statusName) => switch (statusName) {
    'draft' => _german ? 'Entwurf' : 'Draft',
    'recorded' => _german ? 'Aufgenommen' : 'Recorded',
    'reviewed' => _german ? 'Geprüft' : 'Reviewed',
    'approved' => _german ? 'Freigegeben' : 'Approved',
    _ => throw ArgumentError.value(statusName, 'statusName'),
  };
  String takeSubtitle({
    required String statusName,
    required bool isCurrent,
    required bool isApproved,
  }) {
    final status = statusLabel(statusName);
    if (isCurrent && !isApproved) {
      return _german
          ? '$status • Aktuelle Auswahl muss freigegeben sein; ändere den Status zu Freigegeben oder leere die Auswahl'
          : '$status • Current selection must be Approved; change to Approved or clear it';
    }
    final parts = <String>[status];
    if (isCurrent) parts.add(currentSelection);
    if (!isApproved) {
      parts.add(
        _german
            ? 'Freigabe vor der Auswahl erforderlich'
            : 'Approval required before selection',
      );
    }
    return parts.join(' • ');
  }

  String get selectedStatusTooltip => _german
      ? 'Leere die Auswahl, bevor du diesen Status änderst'
      : 'Clear the selection before changing this status';
  String get changeStatusTooltip =>
      _german ? 'Take-Status ändern' : 'Change take status';
  String get changeStatusLabel =>
      _german ? 'Status ändern…' : 'Change status...';
  String get selectedApprovedStatusGuard => _german
      ? 'Leere die Auswahl, bevor du den Status dieses Takes von Freigegeben änderst.'
      : 'Clear the selection before changing this take from Approved.';
}

/// Friendly project-only editor for selecting or clearing an existing Voice
/// take. Entity IDs, CAS heads, paths, and archive internals are never shown.
class Revision3VoiceTakeSelectionDialog extends StatefulWidget {
  const Revision3VoiceTakeSelectionDialog({
    super.key,
    required this.service,
    required this.statusService,
    required this.removalService,
    required this.slotRemovalService,
    required this.copy,
    this.previewMaterialize,
    this.previewPlayback,
    this.initialLineId,
    this.initialLocale,
    this.fixedContext = false,
  }) : assert(
         (previewMaterialize == null) == (previewPlayback == null),
         'Preview authoring and playback must be supplied together.',
       );

  final Revision3VoiceTakeSelectionAuthoringService service;
  final Revision3VoiceTakeStatusAuthoringService statusService;
  final Revision3VoiceTakeRemovalAuthoringService removalService;
  final Revision3DialogVoiceSlotRemovalAuthoringService slotRemovalService;
  final Revision3VoiceTakePreviewDialogMaterializer? previewMaterialize;
  final Revision3VoiceTakePreviewPlaybackController? previewPlayback;
  final String? initialLineId;
  final String? initialLocale;
  final Revision3VoiceTakeSelectionDialogCopy copy;

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
  bool _previewCleanupLocked = false;
  bool _previewStopInFlight = false;
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
      _previewCleanupLocked ||
      _previewStopInFlight ||
      !_fixedContextIsCurrent;

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_searchChanged);
    widget.previewPlayback?.addListener(_previewPlaybackChanged);
    _load();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTakeSelectionDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.previewPlayback, widget.previewPlayback)) {
      oldWidget.previewPlayback?.removeListener(_previewPlaybackChanged);
      widget.previewPlayback?.addListener(_previewPlaybackChanged);
    }
    if (!identical(oldWidget.service, widget.service) ||
        !identical(oldWidget.statusService, widget.statusService) ||
        !identical(oldWidget.removalService, widget.removalService) ||
        !identical(oldWidget.slotRemovalService, widget.slotRemovalService) ||
        !identical(oldWidget.previewMaterialize, widget.previewMaterialize) ||
        !identical(oldWidget.previewPlayback, widget.previewPlayback) ||
        oldWidget.fixedContext != widget.fixedContext ||
        oldWidget.initialLineId != widget.initialLineId ||
        oldWidget.initialLocale != widget.initialLocale) {
      _initialSelectionConsumed = false;
      unawaited(oldWidget.previewPlayback?.stop());
      _load(resetRecovery: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration++;
    widget.previewPlayback?.removeListener(_previewPlaybackChanged);
    unawaited(widget.previewPlayback?.stop());
    _searchController
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() {
    if (mounted) setState(() {});
  }

  void _previewPlaybackChanged() {
    if (!mounted) return;
    switch (widget.previewPlayback?.snapshot.failure) {
      case Revision3VoiceTakePreviewFailureKind.requiresReopen:
        if (_requiresClose &&
            !_reloadRequired &&
            _error == widget.copy.previewRequiresReopen) {
          return;
        }
        setState(() {
          _previewCleanupLocked = false;
          _requiresClose = true;
          _reloadRequired = false;
          _error = widget.copy.previewRequiresReopen;
        });
        break;
      case Revision3VoiceTakePreviewFailureKind.staleCheckpoint:
        if (_requiresClose ||
            (_reloadRequired && _error == widget.copy.previewStale)) {
          return;
        }
        setState(() {
          _previewCleanupLocked = false;
          _reloadRequired = true;
          _error = widget.copy.previewStale;
        });
        break;
      case Revision3VoiceTakePreviewFailureKind.cleanup:
        if (_previewCleanupLocked) return;
        setState(() {
          _previewCleanupLocked = true;
          _notice = null;
        });
        break;
      case null ||
          Revision3VoiceTakePreviewFailureKind.materialize ||
          Revision3VoiceTakePreviewFailureKind.playback:
        break;
    }
  }

  Future<void> _stopPreview() async {
    final playback = widget.previewPlayback;
    if (playback == null || _busy || _previewStopInFlight) return;
    setState(() => _previewStopInFlight = true);
    var stopFailed = false;
    try {
      await playback.stop();
    } catch (_) {
      stopFailed = true;
    }
    if (!mounted) return;
    if (!identical(playback, widget.previewPlayback)) {
      setState(() => _previewStopInFlight = false);
      return;
    }
    final failure = stopFailed
        ? Revision3VoiceTakePreviewFailureKind.cleanup
        : playback.snapshot.failure;
    setState(() {
      _previewStopInFlight = false;
      if (failure == Revision3VoiceTakePreviewFailureKind.cleanup) {
        _previewCleanupLocked = true;
        _notice = null;
        return;
      }
      _previewCleanupLocked = false;
      switch (failure) {
        case Revision3VoiceTakePreviewFailureKind.staleCheckpoint:
          _reloadRequired = true;
          _error = widget.copy.previewStale;
          break;
        case Revision3VoiceTakePreviewFailureKind.requiresReopen:
          _requiresClose = true;
          _reloadRequired = false;
          _error = widget.copy.previewRequiresReopen;
          break;
        case null ||
            Revision3VoiceTakePreviewFailureKind.materialize ||
            Revision3VoiceTakePreviewFailureKind.playback ||
            Revision3VoiceTakePreviewFailureKind.cleanup:
          break;
      }
    });
  }

  Future<bool> _stopPreviewBeforeMutation() async {
    final playback = widget.previewPlayback;
    if (playback == null) return true;
    try {
      await playback.stop();
    } catch (_) {
      if (mounted && identical(playback, widget.previewPlayback)) {
        setState(() {
          _previewCleanupLocked = true;
          _notice = null;
        });
      }
      return false;
    }
    if (!mounted || !identical(playback, widget.previewPlayback)) return false;
    final failure = playback.snapshot.failure;
    if (failure == null) return true;
    setState(() {
      switch (failure) {
        case Revision3VoiceTakePreviewFailureKind.cleanup:
          _previewCleanupLocked = true;
          _notice = null;
          break;
        case Revision3VoiceTakePreviewFailureKind.staleCheckpoint:
          _previewCleanupLocked = false;
          _reloadRequired = true;
          _error = widget.copy.previewStale;
          break;
        case Revision3VoiceTakePreviewFailureKind.requiresReopen:
          _previewCleanupLocked = false;
          _requiresClose = true;
          _reloadRequired = false;
          _error = widget.copy.previewRequiresReopen;
          break;
        case Revision3VoiceTakePreviewFailureKind.materialize ||
            Revision3VoiceTakePreviewFailureKind.playback:
          break;
      }
    });
    return false;
  }

  Future<void> _load({bool resetRecovery = false}) async {
    if (_catalog != null) unawaited(widget.previewPlayback?.stop());
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
          _error = widget.copy.fixedContextUnavailable;
        }
        _loading = false;
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalogLoadFailed = true;
        _requiresClose = true;
        _error = widget.copy.requiresReopen;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalogLoadFailed = true;
        _error = widget.copy.loadFailed;
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
    unawaited(widget.previewPlayback?.stop());
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
    unawaited(widget.previewPlayback?.stop());
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

  Future<void> _previewTake(Revision3VoiceCandidateTake take) async {
    final playback = widget.previewPlayback;
    final materialize = widget.previewMaterialize;
    final catalog = _catalog;
    final lineId = _lineId;
    final locale = _locale;
    if (_interactionLocked ||
        !take.canPreview ||
        playback == null ||
        materialize == null ||
        catalog == null ||
        lineId == null ||
        locale == null) {
      return;
    }
    await playback.preview(
      takeKey: take.id,
      materialize: () async {
        return materialize(
          checkpoint: catalog,
          lineId: lineId,
          locale: locale,
          takeId: take.id,
        );
      },
    );
    if (!mounted || !playback.snapshot.isActive(take.id)) return;
    switch (playback.snapshot.failure) {
      case Revision3VoiceTakePreviewFailureKind.staleCheckpoint:
        setState(() {
          _previewCleanupLocked = false;
          _reloadRequired = true;
          _error = widget.copy.previewStale;
        });
        break;
      case Revision3VoiceTakePreviewFailureKind.requiresReopen:
        setState(() {
          _previewCleanupLocked = false;
          _requiresClose = true;
          _reloadRequired = false;
          _error = widget.copy.previewRequiresReopen;
        });
        break;
      case Revision3VoiceTakePreviewFailureKind.cleanup:
        setState(() {
          _previewCleanupLocked = true;
          _notice = null;
        });
        break;
      case null ||
          Revision3VoiceTakePreviewFailureKind.materialize ||
          Revision3VoiceTakePreviewFailureKind.playback:
        break;
    }
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
      setState(() => _busy = false);
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTakeSelectionStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _requiresClose = true;
        _error = widget.copy.staleSelection;
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _requiresClose = true;
        _error = widget.copy.requiresReopen;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = widget.copy.selectionSaveFailed;
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
    if (!await _stopPreviewBeforeMutation()) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
      });
      return;
    }
    if (!mounted) return;
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
            ? widget.copy.statusChangedApproved
            : widget.copy.statusChanged(
                widget.copy.statusLabel(publication.status.name),
              );
      });
    } on Revision3VoiceTakeStatusSelectedTakeException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _error = widget.copy.selectedTakeStatusBlocked;
      });
    } on Revision3VoiceTakeStatusStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _reloadRequired = true;
        _error = widget.copy.statusStale;
      });
    } on Revision3VoiceTakeStatusRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _requiresClose = true;
        _reloadRequired = false;
        _error = publishedThisAttempt
            ? widget.copy.statusSavedUnconfirmed
            : widget.copy.statusUnconfirmed;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _statusBusyTakeId = null;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? widget.copy.statusSavedReloadFailed
            : widget.copy.statusSaveFailed;
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
        return AlertDialog(
          key: const Key('voice-take-remove-confirm-dialog'),
          title: Text(widget.copy.takeRemoveDialogTitle),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.copy.takeRemoveDialogSummary(
                      take.displayLabel,
                      line.displayLabel,
                      locale,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Text(widget.copy.takeRemoveScope),
                  const SizedBox(height: 8),
                  Text(widget.copy.takeRemoveInternalRetention),
                  const SizedBox(height: 8),
                  Text(widget.copy.takeRemoveGameBoundary),
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
                              widget.copy.takeRemoveSelectedWarning,
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
              child: Text(widget.copy.takeRemoveCancel),
            ),
            FilledButton(
              key: const Key('voice-take-remove-confirm'),
              onPressed: () => Navigator.of(dialogContext).pop(true),
              child: Text(widget.copy.takeRemoveConfirm),
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
    if (!await _stopPreviewBeforeMutation()) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
      });
      return;
    }
    if (!mounted) return;
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
      setState(() {
        _catalog = refreshed;
        _lineId = refreshedLine.lineId;
        _locale = locale;
        _selectionValue = _selectionValueFor(refreshedSummary);
        _busy = false;
        _removalBusyTakeId = null;
        _pendingRemovalPublication = null;
        final outcome = publication!.takeEntityRemoved
            ? widget.copy.takeRemoveUniqueSuccess
            : widget.copy.takeRemoveSharedSuccess;
        _notice = publication.selectionCleared
            ? '$outcome\n${widget.copy.takeRemoveSelectionClearedSuccess}'
            : outcome;
      });
    } on Revision3VoiceTakeRemovalStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = true;
        _error = widget.copy.takeRemoveStale;
      });
    } on Revision3VoiceTakeRemovalRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = false;
        _requiresClose = true;
        _error = publishedThisAttempt
            ? widget.copy.takeRemoveSavedUnconfirmed
            : widget.copy.takeRemoveRequiresReopen;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _removalBusyTakeId = null;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? widget.copy.takeRemoveSavedReloadFailed
            : widget.copy.takeRemoveFailed;
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
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('voice-slot-remove-confirm-dialog'),
        title: Text(widget.copy.slotRemoveDialogTitle),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  widget.copy.slotRemoveDialogSummary(
                    line.displayLabel,
                    locale,
                  ),
                ),
                const SizedBox(height: 12),
                Text(widget.copy.slotRemoveRetention),
                if (retainsTargetEvidence) ...[
                  const SizedBox(height: 12),
                  Container(
                    key: const Key('voice-slot-remove-target-warning'),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Theme.of(dialogContext).colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(widget.copy.slotRemoveTargetWarning),
                  ),
                ],
                const SizedBox(height: 12),
                Text(widget.copy.slotRemoveRecreate),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('voice-slot-remove-cancel'),
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(widget.copy.slotRemoveCancel),
          ),
          FilledButton(
            key: const Key('voice-slot-remove-confirm'),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(widget.copy.slotRemoveConfirm),
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
        _notice = widget.copy.slotRemoveSuccess;
      });
    } on Revision3DialogVoiceSlotRemovalStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _reloadRequired = true;
        _error = widget.copy.slotRemoveStale;
      });
    } on Revision3DialogVoiceSlotRemovalRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = widget.copy.slotRemoveSavedUnconfirmed;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _reloadRequired = publishedThisAttempt;
        _error = publishedThisAttempt
            ? widget.copy.slotRemoveSavedReloadFailed
            : widget.copy.slotRemoveFailed;
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
            _error = widget.copy.fixedContextUnavailable;
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
            ? widget.copy.slotRemoveReloadConfirmed
            : pendingRemoval != null
            ? widget.copy.takeRemoveReloadConfirmed
            : pending == null
            ? widget.copy.latestTakesReloaded
            : widget.copy.savedStatusConfirmed;
      });
    } on Revision3VoiceTakeRemovalRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = widget.copy.takeRemoveSavedUnconfirmed;
      });
    } on Revision3DialogVoiceSlotRemovalRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = widget.copy.slotRemoveSavedUnconfirmed;
      });
    } on Revision3VoiceTakeStatusRequiresReopenException {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _reloadRequired = false;
        _requiresClose = true;
        _error = widget.copy.latestTakesUnconfirmed;
      });
    } catch (_) {
      if (!recoveryIsCurrent()) return;
      setState(() {
        _busy = false;
        _error = widget.copy.reloadFailed;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final copy = widget.copy;
    final lines = _visibleLines;
    final line = _selectedLine;
    final summary = _selectedSummary;
    final previewCleanupNeedsBanner =
        _previewCleanupLocked &&
        widget.previewPlayback?.snapshot.failure !=
            Revision3VoiceTakePreviewFailureKind.cleanup;
    final visibleError = previewCleanupNeedsBanner
        ? copy.previewCleanupFailed
        : _error;
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('revision3-voice-take-selection-dialog'),
        title: Text(copy.title),
        content: SizedBox(
          width: 760,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(copy.introduction),
                const SizedBox(height: 16),
                if (widget.fixedContext && line != null && _locale != null) ...[
                  _SelectionFixedContextBreadcrumb(
                    lineLabel: line.displayLabel,
                    locale: _locale!,
                    copy: copy,
                  ),
                  const SizedBox(height: 8),
                ],
                if (!widget.fixedContext) ...[
                  TextField(
                    key: const Key('voice-selection-line-search'),
                    controller: _searchController,
                    enabled: !_loading && !_interactionLocked,
                    decoration: InputDecoration(
                      labelText: copy.findLineLabel,
                      hintText: copy.findLineHint,
                      prefixIcon: const Icon(Icons.search),
                      border: const OutlineInputBorder(),
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
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 16),
                    child: Text(
                      copy.noMatchingLine,
                      key: const Key('voice-selection-no-lines'),
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
                              copy.voiceLanguageCount(
                                _intactLocales(choice).length,
                              ),
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
                    decoration: InputDecoration(
                      labelText: copy.voiceLanguageLabel,
                      border: const OutlineInputBorder(),
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
                  Text(
                    copy.selectedTakeTitle,
                    style: theme.textTheme.titleMedium,
                  ),
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
                          title: Text(copy.noTakeSelected),
                          subtitle: Text(
                            summary.selectedTakeId == null
                                ? copy.currentSelection
                                : copy.clearActiveChoice,
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
                                _statusBusyTakeId ==
                                summary.candidates[index].id,
                            onStatusChanged: (status) => _changeStatus(
                              summary.candidates[index],
                              status,
                            ),
                            copy: copy,
                            removeBusy:
                                _removalBusyTakeId ==
                                summary.candidates[index].id,
                            onRemove: () =>
                                _confirmRemove(summary.candidates[index]),
                            previewPlayback: widget.previewPlayback,
                            previewEnabled: !_interactionLocked,
                            previewStopEnabled: !_busy && !_previewStopInFlight,
                            onPreview: () =>
                                _previewTake(summary.candidates[index]),
                            onPreviewStop: () => unawaited(_stopPreview()),
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
                        copy.clearSelectionWarning,
                        style: TextStyle(
                          color: theme.colorScheme.onErrorContainer,
                        ),
                      ),
                    ),
                  if (_hasChange)
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: Text(
                        copy.pendingSelectionStatus,
                        key: const Key('voice-status-selection-pending'),
                      ),
                    ),
                  if (summary.candidates.isEmpty)
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            copy.noTakes,
                            key: const Key('voice-selection-no-candidates'),
                          ),
                          if (summary.isRemovableGeneratedSlot) ...[
                            const SizedBox(height: 8),
                            OutlinedButton.icon(
                              key: const Key('voice-slot-remove-empty'),
                              onPressed: _interactionLocked || _hasChange
                                  ? null
                                  : _confirmRemoveEmptySlot,
                              icon: const Icon(Icons.link_off),
                              label: Text(copy.slotRemoveAction),
                            ),
                          ],
                        ],
                      ),
                    ),
                ],
                if (visibleError != null) ...[
                  const SizedBox(height: 12),
                  Text(
                    visibleError,
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
              child: Text(copy.retry),
            ),
          if (_reloadRequired)
            TextButton(
              key: const Key('voice-status-reload'),
              onPressed: _busy ? null : _reloadTakes,
              child: Text(copy.reloadTakes),
            ),
          TextButton(
            key: const Key('voice-selection-cancel'),
            onPressed: _busy ? null : () => Navigator.of(context).pop(),
            child: Text(
              _statusWasSaved ||
                      _removalWasSaved ||
                      _slotRemovalWasSaved ||
                      _requiresClose ||
                      _reloadRequired ||
                      _previewCleanupLocked
                  ? copy.close
                  : copy.cancel,
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
                : Text(copy.saveSelection),
          ),
        ],
      ),
    );
  }
}

class _SelectionFixedContextBreadcrumb extends StatelessWidget {
  const _SelectionFixedContextBreadcrumb({
    required this.lineLabel,
    required this.locale,
    required this.copy,
  });

  final String lineLabel;
  final String locale;
  final Revision3VoiceTakeSelectionDialogCopy copy;

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
              Text(copy.fixedContextLanguage(locale)),
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
    required this.copy,
    required this.removeBusy,
    required this.onRemove,
    required this.previewPlayback,
    required this.previewEnabled,
    required this.previewStopEnabled,
    required this.onPreview,
    required this.onPreviewStop,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;
  final bool statusDisabled;
  final bool statusBusy;
  final ValueChanged<AuthoringRevision3VoiceTakeStatus> onStatusChanged;
  final Revision3VoiceTakeSelectionDialogCopy copy;
  final bool removeBusy;
  final VoidCallback onRemove;
  final Revision3VoiceTakePreviewPlaybackController? previewPlayback;
  final bool previewEnabled;
  final bool previewStopEnabled;
  final VoidCallback onPreview;
  final VoidCallback onPreviewStop;

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
          copy.takeSubtitle(
            statusName: take.status.name,
            isCurrent: isCurrent,
            isApproved: take.isApproved,
          ),
        ),
      ),
      if (previewPlayback != null)
        _TakePreviewControl(
          index: index,
          take: take,
          playback: previewPlayback!,
          enabled: previewEnabled,
          stopEnabled: previewStopEnabled,
          onPreview: onPreview,
          onStop: onPreviewStop,
          copy: copy,
        ),
      _TakeStatusControl(
        index: index,
        take: take,
        isCurrent: isCurrent,
        busy: busy,
        statusDisabled: statusDisabled,
        statusBusy: statusBusy,
        onStatusChanged: onStatusChanged,
        copy: copy,
        removeBusy: removeBusy,
        onRemove: onRemove,
      ),
    ],
  );
}

class _TakePreviewControl extends StatelessWidget {
  const _TakePreviewControl({
    required this.index,
    required this.take,
    required this.playback,
    required this.enabled,
    required this.stopEnabled,
    required this.onPreview,
    required this.onStop,
    required this.copy,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final Revision3VoiceTakePreviewPlaybackController playback;
  final bool enabled;
  final bool stopEnabled;
  final VoidCallback onPreview;
  final VoidCallback onStop;
  final Revision3VoiceTakeSelectionDialogCopy copy;

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: playback,
    builder: (context, _) {
      final snapshot = playback.snapshot;
      final active = snapshot.isActive(take.id);
      if (!take.canPreview) {
        return Padding(
          padding: const EdgeInsets.only(left: 4, right: 4, bottom: 8),
          child: Row(
            children: [
              const Icon(Icons.volume_off_outlined, size: 18),
              const SizedBox(width: 8),
              Flexible(
                child: Text(
                  copy.previewUnavailable,
                  key: ValueKey('voice-preview-unavailable-$index'),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
            ],
          ),
        );
      }
      if (!active) {
        return Align(
          alignment: AlignmentDirectional.centerStart,
          child: Padding(
            padding: const EdgeInsets.only(left: 4, right: 4, bottom: 8),
            child: TextButton.icon(
              key: ValueKey('voice-preview-start-$index'),
              onPressed: enabled ? onPreview : null,
              icon: const Icon(Icons.play_arrow),
              label: Text(copy.previewAction),
            ),
          ),
        );
      }
      return _ActiveTakePreviewControl(
        index: index,
        snapshot: snapshot,
        playback: playback,
        enabled: enabled,
        stopEnabled: stopEnabled,
        onPreview: onPreview,
        onStop: onStop,
        copy: copy,
      );
    },
  );
}

class _ActiveTakePreviewControl extends StatelessWidget {
  const _ActiveTakePreviewControl({
    required this.index,
    required this.snapshot,
    required this.playback,
    required this.enabled,
    required this.stopEnabled,
    required this.onPreview,
    required this.onStop,
    required this.copy,
  });

  final int index;
  final Revision3VoiceTakePreviewPlaybackSnapshot snapshot;
  final Revision3VoiceTakePreviewPlaybackController playback;
  final bool enabled;
  final bool stopEnabled;
  final VoidCallback onPreview;
  final VoidCallback onStop;
  final Revision3VoiceTakeSelectionDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final phase = snapshot.phase;
    final durationMs = snapshot.duration.inMilliseconds;
    final positionMs = snapshot.position.inMilliseconds.clamp(
      0,
      durationMs > 0 ? durationMs : 0,
    );
    final canSeek = enabled && durationMs > 0;
    final failure = snapshot.failure;
    return Container(
      key: ValueKey('voice-preview-active-$index'),
      margin: const EdgeInsets.only(left: 4, right: 4, bottom: 8),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (phase == Revision3VoiceTakePreviewPlaybackPhase.preparing)
            Row(
              children: [
                const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    copy.previewPreparing,
                    key: ValueKey('voice-preview-preparing-$index'),
                  ),
                ),
                _stopButton(),
              ],
            )
          else if (phase == Revision3VoiceTakePreviewPlaybackPhase.failed)
            Wrap(
              spacing: 8,
              runSpacing: 4,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Text(switch (failure) {
                  Revision3VoiceTakePreviewFailureKind.cleanup =>
                    copy.previewCleanupFailed,
                  Revision3VoiceTakePreviewFailureKind.staleCheckpoint =>
                    copy.previewStale,
                  Revision3VoiceTakePreviewFailureKind.requiresReopen =>
                    copy.previewRequiresReopen,
                  _ => copy.previewFailed,
                }, key: ValueKey('voice-preview-error-$index')),
                if (failure !=
                        Revision3VoiceTakePreviewFailureKind.staleCheckpoint &&
                    failure !=
                        Revision3VoiceTakePreviewFailureKind.requiresReopen)
                  TextButton.icon(
                    key: ValueKey('voice-preview-retry-$index'),
                    onPressed: enabled ? onPreview : null,
                    icon: const Icon(Icons.refresh),
                    label: Text(copy.retry),
                  ),
                _stopButton(),
              ],
            )
          else
            Row(
              children: [
                IconButton(
                  key: ValueKey('voice-preview-toggle-$index'),
                  onPressed: enabled
                      ? () => unawaited(
                          phase ==
                                  Revision3VoiceTakePreviewPlaybackPhase.playing
                              ? playback.pause()
                              : playback.play(),
                        )
                      : null,
                  tooltip: switch (phase) {
                    Revision3VoiceTakePreviewPlaybackPhase.playing =>
                      copy.previewPause,
                    Revision3VoiceTakePreviewPlaybackPhase.completed =>
                      copy.previewReplay,
                    _ => copy.previewResume,
                  },
                  icon: Icon(
                    phase == Revision3VoiceTakePreviewPlaybackPhase.playing
                        ? Icons.pause
                        : phase ==
                              Revision3VoiceTakePreviewPlaybackPhase.completed
                        ? Icons.replay
                        : Icons.play_arrow,
                  ),
                ),
                Expanded(
                  child: Slider(
                    key: ValueKey('voice-preview-progress-$index'),
                    value: durationMs > 0 ? positionMs.toDouble() : 0,
                    max: durationMs > 0 ? durationMs.toDouble() : 1,
                    onChanged: canSeek
                        ? (value) => unawaited(
                            playback.seek(
                              Duration(milliseconds: value.round()),
                            ),
                          )
                        : null,
                  ),
                ),
                Text(
                  '${_formatDuration(snapshot.position)} / '
                  '${_formatDuration(snapshot.duration)}',
                  key: ValueKey('voice-preview-time-$index'),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                _stopButton(),
              ],
            ),
        ],
      ),
    );
  }

  Widget _stopButton() => IconButton(
    key: ValueKey('voice-preview-stop-$index'),
    onPressed: stopEnabled ? onStop : null,
    tooltip: copy.previewStop,
    icon: const Icon(Icons.stop),
  );

  static String _formatDuration(Duration value) {
    final totalSeconds = value.inSeconds < 0 ? 0 : value.inSeconds;
    final minutes = totalSeconds ~/ 60;
    final seconds = totalSeconds % 60;
    return '$minutes:${seconds.toString().padLeft(2, '0')}';
  }
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
    required this.copy,
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
  final Revision3VoiceTakeSelectionDialogCopy copy;
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
                    ? copy.selectedStatusTooltip
                    : copy.changeStatusTooltip,
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
                      child: Text(copy.statusLabel(status.name)),
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
                        copy.changeStatusLabel,
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
                message: copy.takeRemoveTooltip,
                child: TextButton.icon(
                  key: ValueKey('voice-take-remove-$index'),
                  onPressed: busy || statusDisabled ? null : onRemove,
                  icon: removeBusy
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.link_off_outlined, size: 18),
                  label: Text(copy.takeRemoveAction),
                ),
              ),
            ],
          ),
        ),
        if (isCurrent &&
            currentStatus == AuthoringRevision3VoiceTakeStatus.approved)
          Padding(
            padding: const EdgeInsets.only(left: 12, bottom: 8),
            child: Text(copy.selectedApprovedStatusGuard),
          ),
      ],
    );
  }
}

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
