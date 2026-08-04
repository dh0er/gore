import 'dart:async';

import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';

import 'revision3_dialog_line_authoring.dart';
import 'revision3_npc_greeting_authoring.dart';

typedef Revision3NpcGreetingCreateLineAction =
    Future<bool> Function({
      required Revision3NpcGreetingProjection projection,
      required int insertionIndex,
      required Revision3DialogLineEntryTechnicalPublisher publishTechnicalPlan,
    });

typedef Revision3NpcGreetingOpenTextVoiceAction =
    Future<bool> Function({
      required Revision3NpcGreetingProjection projection,
      required Revision3NpcGreetingRow row,
      required String locale,
    });

typedef Revision3NpcGreetingPublishedAction =
    Future<void> Function(Revision3NpcGreetingPublication publication);

@immutable
final class Revision3NpcDialogVoicePanelCopy {
  const Revision3NpcDialogVoicePanelCopy({
    required this.title,
    required this.description,
    required this.scopeNotice,
    required this.loading,
    required this.loadFailedTitle,
    required this.loadFailedDescription,
    required this.retry,
    required this.newLine,
    required this.editGreetings,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.lineCountTemplate,
    required this.speakerTemplate,
    required this.noSpeaker,
    required this.localesTemplate,
    required this.noLocales,
    required this.textCoverageTemplate,
    required this.voiceCoverageTemplate,
    required this.selectedVoiceTemplate,
    required this.previewTitle,
    required this.previewLoading,
    required this.previewFailed,
    required this.previewEmpty,
    required this.previewTruncated,
    required this.localeLabel,
    required this.openTextVoice,
    required this.openingTextVoice,
    required this.openFailed,
    required this.reviewTitle,
    required this.reviewDescription,
    required this.attachExisting,
    required this.noUnboundLines,
    required this.moveUp,
    required this.moveDown,
    required this.removeFromGreetings,
    required this.cancel,
    required this.save,
    required this.saving,
    required this.discardTitle,
    required this.discardDescription,
    required this.keepEditing,
    required this.discard,
    required this.mutationDisabledFallback,
    required this.requiresReopen,
    required this.waitingForRefresh,
    required this.saveFailed,
    required this.createFailed,
    required this.maximumReached,
  });

  static const english = Revision3NpcDialogVoicePanelCopy(
    title: 'Greeting lines',
    description:
        'Arrange the lines this NPC can use as authored greetings, then review text and Voice coverage.',
    scopeNotice:
        'Authoring metadata only: this does not create an AngelScript topic or make the greeting playable in-game yet.',
    loading: 'Opening the exact NPC greetings\u2026',
    loadFailedTitle: 'NPC greetings unavailable',
    loadFailedDescription:
        'The exact current greeting list could not be verified. Refresh the project before editing.',
    retry: 'Retry',
    newLine: 'New greeting line',
    editGreetings: 'Edit greetings',
    emptyTitle: 'No greeting lines yet',
    emptyDescription:
        'Create the first greeting line or attach an existing unbound project line.',
    lineCountTemplate: '{count} greeting lines',
    speakerTemplate: 'Speaker: {speaker}',
    noSpeaker: 'Speaker not set',
    localesTemplate: 'Languages: {locales}',
    noLocales: 'No authored language',
    textCoverageTemplate: 'Text {authored}/{total}',
    voiceCoverageTemplate: 'Voice {slots}/{total} \u00b7 {takes} takes',
    selectedVoiceTemplate: '{count} selected',
    previewTitle: 'Localized text',
    previewLoading: 'Loading exact text preview\u2026',
    previewFailed: 'This text preview could not be verified.',
    previewEmpty: 'No authored text in this language.',
    previewTruncated: 'Preview shortened',
    localeLabel: 'Language',
    openTextVoice: 'Open text & Voice',
    openingTextVoice: 'Opening\u2026',
    openFailed: 'The exact text and Voice target could not be opened.',
    reviewTitle: 'Edit NPC greetings',
    reviewDescription:
        'Attach existing project lines, change their order, or remove lines from this NPC greeting list.',
    attachExisting: 'Attach existing line',
    noUnboundLines: 'No unbound project lines are available.',
    moveUp: 'Move up',
    moveDown: 'Move down',
    removeFromGreetings: 'Remove from greetings',
    cancel: 'Cancel',
    save: 'Save greetings',
    saving: 'Saving\u2026',
    discardTitle: 'Discard greeting changes?',
    discardDescription:
        'The unsaved order, attachments, and removals will be lost.',
    keepEditing: 'Keep editing',
    discard: 'Discard changes',
    mutationDisabledFallback: 'Greeting editing is currently unavailable.',
    requiresReopen:
        'The project changed or exact authority was lost. Refresh or reopen the project before editing.',
    waitingForRefresh: 'Published. Waiting for the refreshed project\u2026',
    saveFailed: 'The greeting list could not be saved.',
    createFailed: 'The new greeting line could not be created.',
    maximumReached: 'This NPC already contains the maximum of 256 greetings.',
  );

  static const german = Revision3NpcDialogVoicePanelCopy(
    title: 'Begr\u00fc\u00dfungszeilen',
    description:
        'Zeilen f\u00fcr die Begr\u00fc\u00dfungen dieses NPCs anordnen und Text- sowie Voice-Abdeckung pr\u00fcfen.',
    scopeNotice:
        'Nur Authoring-Metadaten: Dadurch entsteht noch kein AngelScript-Thema und die Begr\u00fc\u00dfung wird noch nicht im Spiel abspielbar.',
    loading: 'Die exakten NPC-Begr\u00fc\u00dfungen werden ge\u00f6ffnet\u2026',
    loadFailedTitle: 'NPC-Begr\u00fc\u00dfungen nicht verf\u00fcgbar',
    loadFailedDescription:
        'Die aktuelle Begr\u00fc\u00dfungsliste konnte nicht eindeutig gepr\u00fcft werden. Aktualisiere das Projekt vor dem Bearbeiten.',
    retry: 'Erneut versuchen',
    newLine: 'Neue Begr\u00fc\u00dfungszeile',
    editGreetings: 'Begr\u00fc\u00dfungen bearbeiten',
    emptyTitle: 'Noch keine Begr\u00fc\u00dfungszeilen',
    emptyDescription:
        'Erstelle die erste Begr\u00fc\u00dfungszeile oder verkn\u00fcpfe eine vorhandene, ungebundene Projektzeile.',
    lineCountTemplate: '{count} Begr\u00fc\u00dfungszeilen',
    speakerTemplate: 'Sprecher: {speaker}',
    noSpeaker: 'Kein Sprecher festgelegt',
    localesTemplate: 'Sprachen: {locales}',
    noLocales: 'Keine Sprache mit Text',
    textCoverageTemplate: 'Text {authored}/{total}',
    voiceCoverageTemplate: 'Voice {slots}/{total} \u00b7 {takes} Aufnahmen',
    selectedVoiceTemplate: '{count} ausgew\u00e4hlt',
    previewTitle: 'Lokalisierter Text',
    previewLoading: 'Exakte Textvorschau wird geladen\u2026',
    previewFailed:
        'Diese Textvorschau konnte nicht eindeutig gepr\u00fcft werden.',
    previewEmpty: 'In dieser Sprache ist noch kein Text vorhanden.',
    previewTruncated: 'Vorschau gek\u00fcrzt',
    localeLabel: 'Sprache',
    openTextVoice: 'Text & Voice \u00f6ffnen',
    openingTextVoice: 'Wird ge\u00f6ffnet\u2026',
    openFailed:
        'Das exakte Text- und Voice-Ziel konnte nicht ge\u00f6ffnet werden.',
    reviewTitle: 'NPC-Begr\u00fc\u00dfungen bearbeiten',
    reviewDescription:
        'Vorhandene Projektzeilen verkn\u00fcpfen, ihre Reihenfolge \u00e4ndern oder sie aus der Begr\u00fc\u00dfungsliste entfernen.',
    attachExisting: 'Vorhandene Zeile verkn\u00fcpfen',
    noUnboundLines: 'Es sind keine ungebundenen Projektzeilen verf\u00fcgbar.',
    moveUp: 'Nach oben',
    moveDown: 'Nach unten',
    removeFromGreetings: 'Aus Begr\u00fc\u00dfungen entfernen',
    cancel: 'Abbrechen',
    save: 'Begr\u00fc\u00dfungen speichern',
    saving: 'Wird gespeichert\u2026',
    discardTitle: 'Begr\u00fc\u00dfungs\u00e4nderungen verwerfen?',
    discardDescription:
        'Ungespeicherte Reihenfolge, Verkn\u00fcpfungen und Entfernungen gehen verloren.',
    keepEditing: 'Weiter bearbeiten',
    discard: '\u00c4nderungen verwerfen',
    mutationDisabledFallback:
        'Die NPC-Begr\u00fc\u00dfungen k\u00f6nnen momentan nicht bearbeitet werden.',
    requiresReopen:
        'Das Projekt hat sich ge\u00e4ndert oder die exakte Berechtigung ging verloren. Aktualisiere oder \u00f6ffne es erneut.',
    waitingForRefresh:
        'Ver\u00f6ffentlicht. Das aktualisierte Projekt wird erwartet\u2026',
    saveFailed:
        'Die Begr\u00fc\u00dfungsliste konnte nicht gespeichert werden.',
    createFailed:
        'Die neue Begr\u00fc\u00dfungszeile konnte nicht erstellt werden.',
    maximumReached:
        'Dieser NPC enth\u00e4lt bereits das Maximum von 256 Begr\u00fc\u00dfungen.',
  );

  final String title;
  final String description;
  final String scopeNotice;
  final String loading;
  final String loadFailedTitle;
  final String loadFailedDescription;
  final String retry;
  final String newLine;
  final String editGreetings;
  final String emptyTitle;
  final String emptyDescription;
  final String lineCountTemplate;
  final String speakerTemplate;
  final String noSpeaker;
  final String localesTemplate;
  final String noLocales;
  final String textCoverageTemplate;
  final String voiceCoverageTemplate;
  final String selectedVoiceTemplate;
  final String previewTitle;
  final String previewLoading;
  final String previewFailed;
  final String previewEmpty;
  final String previewTruncated;
  final String localeLabel;
  final String openTextVoice;
  final String openingTextVoice;
  final String openFailed;
  final String reviewTitle;
  final String reviewDescription;
  final String attachExisting;
  final String noUnboundLines;
  final String moveUp;
  final String moveDown;
  final String removeFromGreetings;
  final String cancel;
  final String save;
  final String saving;
  final String discardTitle;
  final String discardDescription;
  final String keepEditing;
  final String discard;
  final String mutationDisabledFallback;
  final String requiresReopen;
  final String waitingForRefresh;
  final String saveFailed;
  final String createFailed;
  final String maximumReached;

  String lineCount(int count) =>
      lineCountTemplate.replaceAll('{count}', '$count');
  String speaker(String value) =>
      speakerTemplate.replaceAll('{speaker}', value);
  String locales(Iterable<String> values) =>
      localesTemplate.replaceAll('{locales}', values.join(', '));
  String textCoverage(int authored, int total) => textCoverageTemplate
      .replaceAll('{authored}', '$authored')
      .replaceAll('{total}', '$total');
  String voiceCoverage(int slots, int total, int takes) => voiceCoverageTemplate
      .replaceAll('{slots}', '$slots')
      .replaceAll('{total}', '$total')
      .replaceAll('{takes}', '$takes');
  String selectedVoice(int count) =>
      selectedVoiceTemplate.replaceAll('{count}', '$count');
}

/// Friendly, responsive editor for the ordered greeting-line bindings of one
/// exact revision-3 NPC draft. Technical identities remain in callbacks only.
final class Revision3NpcDialogVoicePanel extends StatefulWidget {
  const Revision3NpcDialogVoicePanel({
    required this.projectId,
    required this.projectRevision,
    required this.projectCheckpointIdentity,
    required this.npcId,
    required this.npcRevision,
    required this.service,
    required this.selectedLineId,
    required this.onSelectedLineChanged,
    required this.onCreateLine,
    required this.onOpenTextVoice,
    this.onPublished,
    this.mutationsEnabled = true,
    this.mutationDisabledReason,
    this.copy = Revision3NpcDialogVoicePanelCopy.english,
    super.key,
  }) : assert(projectId != ''),
       assert(projectRevision >= 1),
       assert(npcId != ''),
       assert(npcRevision >= 0),
       assert(
         mutationsEnabled ||
             (mutationDisabledReason != null && mutationDisabledReason != ''),
       );

  final String projectId;
  final int projectRevision;
  final Object projectCheckpointIdentity;
  final String npcId;
  final int npcRevision;
  final Revision3NpcGreetingAuthoringService service;
  final String? selectedLineId;
  final ValueChanged<String?> onSelectedLineChanged;
  final Revision3NpcGreetingCreateLineAction onCreateLine;
  final Revision3NpcGreetingOpenTextVoiceAction onOpenTextVoice;
  final Revision3NpcGreetingPublishedAction? onPublished;
  final bool mutationsEnabled;
  final String? mutationDisabledReason;
  final Revision3NpcDialogVoicePanelCopy copy;

  @override
  State<Revision3NpcDialogVoicePanel> createState() =>
      _Revision3NpcDialogVoicePanelState();
}

class _Revision3NpcDialogVoicePanelState
    extends State<Revision3NpcDialogVoicePanel> {
  Revision3NpcGreetingProjection? _projection;
  Revision3NpcGreetingTextPreview? _preview;
  Object? _loadError;
  String? _selectedLineId;
  String? _selectedLocale;
  bool _loading = true;
  bool _previewLoading = false;
  bool _busy = false;
  bool _opening = false;
  bool _requiresReopen = false;
  bool _waitingForRefresh = false;
  String? _actionMessage;
  int _loadEpoch = 0;
  int _previewEpoch = 0;
  int _actionEpoch = 0;
  late final ValueNotifier<String?> _reviewMutationBlock;
  Route<Revision3NpcGreetingPublication?>? _reviewRoute;

  @override
  void initState() {
    super.initState();
    _selectedLineId = widget.selectedLineId;
    _reviewMutationBlock = ValueNotifier<String?>(_mutationBlockReason);
    unawaited(_load());
  }

  @override
  void didUpdateWidget(covariant Revision3NpcDialogVoicePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selectedLineId != widget.selectedLineId) {
      _selectedLineId = widget.selectedLineId;
      _reconcileSelection(loadPreview: true);
    }
    final checkpointChanged =
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectCheckpointIdentity !=
            widget.projectCheckpointIdentity ||
        oldWidget.npcId != widget.npcId ||
        oldWidget.npcRevision != widget.npcRevision;
    if (checkpointChanged) {
      final route = _reviewRoute;
      if (route != null && route.isActive) route.navigator?.removeRoute(route);
      _reviewRoute = null;
      _actionEpoch++;
      _requiresReopen = false;
      _waitingForRefresh = false;
      _actionMessage = null;
      unawaited(_load(clear: true));
    }
    _syncReviewMutationBlock();
  }

  @override
  void dispose() {
    _loadEpoch++;
    _previewEpoch++;
    _actionEpoch++;
    final route = _reviewRoute;
    if (route != null && route.isActive) route.navigator?.removeRoute(route);
    _reviewMutationBlock.dispose();
    super.dispose();
  }

  String? get _mutationBlockReason {
    if (_requiresReopen) return widget.copy.requiresReopen;
    if (_waitingForRefresh) return widget.copy.waitingForRefresh;
    if (!widget.mutationsEnabled) {
      return widget.mutationDisabledReason ??
          widget.copy.mutationDisabledFallback;
    }
    return null;
  }

  void _syncReviewMutationBlock() {
    final next = _mutationBlockReason;
    if (_reviewMutationBlock.value != next) _reviewMutationBlock.value = next;
  }

  bool _matchesWidget(Revision3NpcGreetingProjection value) =>
      value.projectId == widget.projectId &&
      value.projectRevision == widget.projectRevision &&
      value.checkpointIdentity == widget.projectCheckpointIdentity &&
      value.npcId == widget.npcId &&
      value.npcRevision == widget.npcRevision;

  Future<void> _load({bool clear = false}) async {
    final epoch = ++_loadEpoch;
    _previewEpoch++;
    setState(() {
      _loading = true;
      _loadError = null;
      _actionMessage = null;
      _preview = null;
      _previewLoading = false;
      if (clear) _projection = null;
    });
    try {
      final projection = await widget.service.load(
        npcId: widget.npcId,
        expectedNpcRevision: widget.npcRevision,
      );
      if (!mounted || epoch != _loadEpoch) return;
      if (!_matchesWidget(projection)) {
        throw const Revision3NpcGreetingStaleCheckpointException();
      }
      setState(() {
        _projection = projection;
        _loading = false;
        _requiresReopen = false;
      });
      _syncReviewMutationBlock();
      _reconcileSelection(loadPreview: true);
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() {
        _loading = false;
        _loadError = error;
        _requiresReopen = _npcGreetingAuthorityError(error);
      });
      _syncReviewMutationBlock();
    }
  }

  Revision3NpcGreetingRow? get _selectedRow {
    final projection = _projection;
    final selected = _selectedLineId;
    if (projection == null || selected == null) return null;
    for (final row in projection.rows) {
      if (row.lineId == selected) return row;
    }
    return null;
  }

  void _reconcileSelection({required bool loadPreview}) {
    final projection = _projection;
    if (projection == null) return;
    var row = _selectedRow;
    if (row == null) {
      row = projection.rows.firstOrNull;
      final nextId = row?.lineId;
      if (_selectedLineId != nextId) {
        _selectedLineId = nextId;
        widget.onSelectedLineChanged(nextId);
      }
    }
    final locales = row == null ? const <String>[] : _npcGreetingLocales(row);
    if (!locales.contains(_selectedLocale)) {
      _selectedLocale = locales.firstOrNull;
    }
    if (loadPreview && row != null) {
      unawaited(_loadPreview(row));
    } else if (row == null) {
      _previewEpoch++;
      setState(() {
        _preview = null;
        _previewLoading = false;
      });
    }
  }

  Future<void> _loadPreview(Revision3NpcGreetingRow row) async {
    final projection = _projection;
    if (projection == null || !identical(_selectedRow, row)) return;
    final epoch = ++_previewEpoch;
    setState(() {
      _previewLoading = true;
      _preview = null;
    });
    try {
      final preview = await widget.service.loadTextPreview(
        projection: projection,
        row: row,
      );
      if (!mounted ||
          epoch != _previewEpoch ||
          !identical(_projection, projection) ||
          !identical(_selectedRow, row)) {
        return;
      }
      setState(() {
        _preview = preview;
        _previewLoading = false;
      });
    } catch (error) {
      if (!mounted ||
          epoch != _previewEpoch ||
          !identical(_projection, projection) ||
          !identical(_selectedRow, row)) {
        return;
      }
      setState(() {
        _preview = null;
        _previewLoading = false;
        if (_npcGreetingAuthorityError(error)) _requiresReopen = true;
      });
      _syncReviewMutationBlock();
    }
  }

  void _selectRow(Revision3NpcGreetingRow row) {
    if (identical(_selectedRow, row)) return;
    _selectedLineId = row.lineId;
    widget.onSelectedLineChanged(row.lineId);
    final locales = _npcGreetingLocales(row);
    setState(() {
      _selectedLocale = locales.firstOrNull;
      _actionMessage = null;
    });
    unawaited(_loadPreview(row));
  }

  Future<void> _createLine() async {
    final projection = _projection;
    if (projection == null ||
        _busy ||
        _mutationBlockReason != null ||
        projection.rows.length >= 256) {
      return;
    }
    final selected = _selectedRow;
    final selectedIndex = selected == null
        ? -1
        : projection.rows.indexOf(selected);
    final insertionIndex = selectedIndex < 0
        ? projection.rows.length
        : selectedIndex + 1;
    final epoch = ++_actionEpoch;
    setState(() {
      _busy = true;
      _actionMessage = null;
    });
    try {
      final exactPublisher = widget.service.createAndInsertPublisher(
        projection: projection,
        index: insertionIndex,
      );
      final published = await widget.onCreateLine(
        projection: projection,
        insertionIndex: insertionIndex,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) {
              if (!mounted ||
                  epoch != _actionEpoch ||
                  !_busy ||
                  !identical(_projection, projection) ||
                  _mutationBlockReason != null) {
                throw const Revision3NpcGreetingStaleCheckpointException();
              }
              return exactPublisher(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                plan: plan,
              );
            },
      );
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _busy = false;
        if (published) _waitingForRefresh = true;
      });
      _syncReviewMutationBlock();
    } catch (error) {
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _busy = false;
        _requiresReopen = _npcGreetingAuthorityError(error);
        _actionMessage = widget.copy.createFailed;
      });
      _syncReviewMutationBlock();
    }
  }

  Future<void> _editGreetings() async {
    final projection = _projection;
    if (projection == null || _busy || _mutationBlockReason != null) return;
    Route<Revision3NpcGreetingPublication?>? route;
    Revision3NpcGreetingPublication? publication;
    try {
      publication = await showDialog<Revision3NpcGreetingPublication>(
        context: context,
        barrierDismissible: false,
        builder: (dialogContext) {
          route ??=
              ModalRoute.of(dialogContext)
                  as Route<Revision3NpcGreetingPublication?>?;
          _reviewRoute = route;
          return _Revision3NpcGreetingReviewDialog(
            projection: projection,
            service: widget.service,
            mutationBlock: _reviewMutationBlock,
            copy: widget.copy,
            onAuthorityLost: _markRequiresReopen,
          );
        },
      );
    } finally {
      if (identical(_reviewRoute, route)) _reviewRoute = null;
    }
    if (!mounted ||
        publication == null ||
        !identical(_projection, projection)) {
      return;
    }
    final epoch = ++_actionEpoch;
    setState(() {
      _busy = true;
      _actionMessage = null;
    });
    try {
      await widget.onPublished?.call(publication);
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _busy = false;
        _waitingForRefresh = true;
      });
      _syncReviewMutationBlock();
    } catch (_) {
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _busy = false;
        _requiresReopen = true;
        _actionMessage = widget.copy.saveFailed;
      });
      _syncReviewMutationBlock();
    }
  }

  void _markRequiresReopen() {
    if (!mounted || _requiresReopen) return;
    setState(() => _requiresReopen = true);
    _syncReviewMutationBlock();
  }

  Future<void> _openTextVoice() async {
    final projection = _projection;
    final row = _selectedRow;
    final locale = _selectedLocale;
    if (projection == null || row == null || locale == null || _opening) return;
    final epoch = ++_actionEpoch;
    setState(() {
      _opening = true;
      _actionMessage = null;
    });
    try {
      final opened = await widget.onOpenTextVoice(
        projection: projection,
        row: row,
        locale: locale,
      );
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _opening = false;
        if (!opened) _actionMessage = widget.copy.openFailed;
      });
    } catch (_) {
      if (!mounted ||
          epoch != _actionEpoch ||
          !identical(_projection, projection)) {
        return;
      }
      setState(() {
        _opening = false;
        _actionMessage = widget.copy.openFailed;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final projection = _projection;
    final mutationBlock = _mutationBlockReason;
    final maxReached = (projection?.rows.length ?? 0) >= 256;
    final media = MediaQuery.sizeOf(context);
    final compact = media.width < 720;
    final panelHeight = (media.height * (compact ? 0.82 : 0.62)).clamp(
      compact ? 360.0 : 300.0,
      680.0,
    );
    return Material(
      key: const Key('revision3-npc-dialog-voice-panel'),
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: Theme.of(context).colorScheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        height: panelHeight,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: panelHeight * 0.52),
              child: SingleChildScrollView(
                key: const Key('revision3-npc-greeting-chrome-scroll'),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _buildHeader(
                      context,
                      projection,
                      mutationBlock,
                      maxReached,
                    ),
                    _NpcGreetingNotice(
                      key: const Key('revision3-npc-greeting-scope'),
                      icon: Icons.info_outline,
                      text: widget.copy.scopeNotice,
                      subtle: true,
                    ),
                    if (mutationBlock != null)
                      _NpcGreetingNotice(
                        key: const Key('revision3-npc-greeting-mutation-block'),
                        icon: Icons.lock_outline,
                        text: mutationBlock,
                      ),
                    if (_actionMessage != null)
                      _NpcGreetingNotice(
                        key: const Key('revision3-npc-greeting-action-message'),
                        icon: Icons.info_outline,
                        text: _actionMessage!,
                      ),
                  ],
                ),
              ),
            ),
            const Divider(height: 1),
            Expanded(child: _buildBody(context)),
          ],
        ),
      ),
    );
  }

  Widget _buildHeader(
    BuildContext context,
    Revision3NpcGreetingProjection? projection,
    String? mutationBlock,
    bool maxReached,
  ) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 14, 12, 12),
    child: Wrap(
      spacing: 12,
      runSpacing: 10,
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Semantics(
                header: true,
                child: Text(
                  widget.copy.title,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              const SizedBox(height: 2),
              Text(widget.copy.description),
              if (projection != null)
                Text(
                  widget.copy.lineCount(projection.rows.length),
                  key: const Key('revision3-npc-greeting-line-count'),
                  style: Theme.of(context).textTheme.labelMedium,
                ),
            ],
          ),
        ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Tooltip(
              message: maxReached
                  ? widget.copy.maximumReached
                  : mutationBlock ?? '',
              child: FilledButton.tonalIcon(
                key: const Key('revision3-npc-greeting-new-line'),
                onPressed:
                    projection == null ||
                        _busy ||
                        maxReached ||
                        mutationBlock != null
                    ? null
                    : _createLine,
                icon: _busy
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.add_comment_outlined),
                label: Text(widget.copy.newLine),
              ),
            ),
            Tooltip(
              message: mutationBlock ?? '',
              child: OutlinedButton.icon(
                key: const Key('revision3-npc-greeting-edit'),
                onPressed: projection == null || _busy || mutationBlock != null
                    ? null
                    : _editGreetings,
                icon: const Icon(Icons.reorder_outlined),
                label: Text(widget.copy.editGreetings),
              ),
            ),
          ],
        ),
      ],
    ),
  );

  Widget _buildBody(BuildContext context) {
    if (_loading && _projection == null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 12),
              Text(widget.copy.loading, textAlign: TextAlign.center),
            ],
          ),
        ),
      );
    }
    if (_loadError != null && _projection == null) {
      return _NpcGreetingLoadError(
        copy: widget.copy,
        retry: _requiresReopen ? null : _load,
      );
    }
    final projection = _projection!;
    if (projection.rows.isEmpty) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.waving_hand_outlined, size: 36),
              const SizedBox(height: 8),
              Text(
                widget.copy.emptyTitle,
                style: Theme.of(context).textTheme.titleMedium,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 4),
              Text(widget.copy.emptyDescription, textAlign: TextAlign.center),
            ],
          ),
        ),
      );
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final list = _NpcGreetingLineList(
          projection: projection,
          selected: _selectedRow,
          copy: widget.copy,
          onSelected: _selectRow,
        );
        final detail = _buildSelectedDetail(context);
        if (constraints.maxWidth >= 720) {
          return Row(
            key: const Key('revision3-npc-greeting-wide'),
            children: [
              Expanded(flex: 5, child: list),
              const VerticalDivider(width: 1),
              Expanded(flex: 6, child: detail),
            ],
          );
        }
        return Column(
          key: const Key('revision3-npc-greeting-compact'),
          children: [
            SizedBox(
              height: (constraints.maxHeight * 0.42).clamp(56.0, 180.0),
              child: list,
            ),
            const Divider(height: 1),
            Expanded(child: detail),
          ],
        );
      },
    );
  }

  Widget _buildSelectedDetail(BuildContext context) {
    final row = _selectedRow;
    if (row == null) return const SizedBox.shrink();
    final locales = _npcGreetingLocales(row);
    final selectedLocale = locales.contains(_selectedLocale)
        ? _selectedLocale
        : locales.firstOrNull;
    final previewLocale = _preview?.locales
        .where((locale) => locale.locale == selectedLocale)
        .firstOrNull;
    return SingleChildScrollView(
      key: const Key('revision3-npc-greeting-detail'),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            row.displayLabel,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 3),
          Text(
            row.speakerLabel == null
                ? widget.copy.noSpeaker
                : widget.copy.speaker(row.speakerLabel!),
          ),
          const SizedBox(height: 12),
          if (locales.isEmpty)
            Text(widget.copy.noLocales)
          else
            KeyedSubtree(
              key: ObjectKey(row),
              child: DropdownButtonFormField<String>(
                key: const Key('revision3-npc-greeting-locale'),
                initialValue: selectedLocale,
                decoration: InputDecoration(
                  labelText: widget.copy.localeLabel,
                  isDense: true,
                ),
                items: [
                  for (final locale in locales)
                    DropdownMenuItem(value: locale, child: Text(locale)),
                ],
                onChanged: (locale) {
                  if (locale != null) setState(() => _selectedLocale = locale);
                },
              ),
            ),
          const SizedBox(height: 12),
          Text(
            widget.copy.previewTitle,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 5),
          Container(
            key: const Key('revision3-npc-greeting-preview'),
            constraints: const BoxConstraints(minHeight: 76),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: _previewLoading
                ? Row(
                    children: [
                      const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      const SizedBox(width: 10),
                      Expanded(child: Text(widget.copy.previewLoading)),
                    ],
                  )
                : _preview == null
                ? Text(widget.copy.previewFailed)
                : previewLocale?.hasNonemptyText != true
                ? Text(widget.copy.previewEmpty)
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      SelectableText(previewLocale!.text),
                      if (previewLocale.truncated) ...[
                        const SizedBox(height: 6),
                        Text(
                          widget.copy.previewTruncated,
                          style: Theme.of(context).textTheme.labelSmall,
                        ),
                      ],
                    ],
                  ),
          ),
          const SizedBox(height: 12),
          Align(
            alignment: AlignmentDirectional.centerEnd,
            child: FilledButton.tonalIcon(
              key: const Key('revision3-npc-greeting-open-text-voice'),
              onPressed: selectedLocale == null || _opening
                  ? null
                  : _openTextVoice,
              icon: _opening
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.record_voice_over_outlined),
              label: Text(
                _opening
                    ? widget.copy.openingTextVoice
                    : widget.copy.openTextVoice,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

List<String> _npcGreetingLocales(Revision3NpcGreetingRow row) {
  final locales = <String>{
    ...row.authoredLocales,
    for (final coverage in row.localeCoverage) coverage.locale,
  }.toList(growable: false)..sort();
  return locales;
}

bool _npcGreetingAuthorityError(Object error) =>
    error is Revision3NpcGreetingRequiresReopenException ||
    error is Revision3NpcGreetingStaleCheckpointException;

class _NpcGreetingLineList extends StatelessWidget {
  const _NpcGreetingLineList({
    required this.projection,
    required this.selected,
    required this.copy,
    required this.onSelected,
  });

  final Revision3NpcGreetingProjection projection;
  final Revision3NpcGreetingRow? selected;
  final Revision3NpcDialogVoicePanelCopy copy;
  final ValueChanged<Revision3NpcGreetingRow> onSelected;

  @override
  Widget build(BuildContext context) => ListView.builder(
    key: const Key('revision3-npc-greeting-lines'),
    padding: const EdgeInsets.symmetric(vertical: 6),
    itemCount: projection.rows.length,
    itemBuilder: (context, index) {
      final row = projection.rows[index];
      final locales = _npcGreetingLocales(row);
      final authoredCount = row.localeCoverage
          .where((locale) => locale.hasAuthoredText)
          .length;
      return Semantics(
        button: true,
        selected: identical(selected, row),
        child: ListTile(
          key: Key('revision3-npc-greeting-row-$index'),
          selected: identical(selected, row),
          leading: CircleAvatar(radius: 15, child: Text('${index + 1}')),
          title: Text(
            row.displayLabel,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                row.speakerLabel == null
                    ? copy.noSpeaker
                    : copy.speaker(row.speakerLabel!),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
              Wrap(
                spacing: 6,
                runSpacing: 3,
                children: [
                  _NpcGreetingCoverageChip(
                    label: locales.isEmpty
                        ? copy.noLocales
                        : copy.locales(locales),
                    icon: Icons.language_outlined,
                  ),
                  _NpcGreetingCoverageChip(
                    label: copy.textCoverage(authoredCount, locales.length),
                    icon: Icons.translate_outlined,
                  ),
                  _NpcGreetingCoverageChip(
                    label: copy.voiceCoverage(
                      row.voiceSlotCount,
                      locales.length,
                      row.voiceTakeCount,
                    ),
                    icon: Icons.mic_none_outlined,
                  ),
                  if (row.selectedVoiceTakeCount > 0)
                    _NpcGreetingCoverageChip(
                      label: copy.selectedVoice(row.selectedVoiceTakeCount),
                      icon: Icons.check_circle_outline,
                    ),
                ],
              ),
            ],
          ),
          onTap: () => onSelected(row),
        ),
      );
    },
  );
}

class _NpcGreetingCoverageChip extends StatelessWidget {
  const _NpcGreetingCoverageChip({required this.label, required this.icon});

  final String label;
  final IconData icon;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 13),
        const SizedBox(width: 4),
        Flexible(
          child: Text(
            label,
            style: Theme.of(context).textTheme.labelSmall,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    ),
  );
}

class _NpcGreetingNotice extends StatelessWidget {
  const _NpcGreetingNotice({
    required this.icon,
    required this.text,
    this.subtle = false,
    super.key,
  });

  final IconData icon;
  final String text;
  final bool subtle;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final background = subtle
        ? colors.surfaceContainer
        : colors.secondaryContainer;
    final foreground = subtle
        ? colors.onSurfaceVariant
        : colors.onSecondaryContainer;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      color: background,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 18, color: foreground),
          const SizedBox(width: 8),
          Expanded(
            child: Text(text, style: TextStyle(color: foreground)),
          ),
        ],
      ),
    );
  }
}

class _NpcGreetingLoadError extends StatelessWidget {
  const _NpcGreetingLoadError({required this.copy, required this.retry});

  final Revision3NpcDialogVoicePanelCopy copy;
  final VoidCallback? retry;

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline, size: 36),
          const SizedBox(height: 8),
          Text(
            copy.loadFailedTitle,
            style: Theme.of(context).textTheme.titleMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 4),
          Text(copy.loadFailedDescription, textAlign: TextAlign.center),
          if (retry != null) ...[
            const SizedBox(height: 12),
            OutlinedButton.icon(
              key: const Key('revision3-npc-greeting-retry'),
              onPressed: retry,
              icon: const Icon(Icons.refresh),
              label: Text(copy.retry),
            ),
          ],
        ],
      ),
    ),
  );
}

class _Revision3NpcGreetingReviewDialog extends StatefulWidget {
  const _Revision3NpcGreetingReviewDialog({
    required this.projection,
    required this.service,
    required this.mutationBlock,
    required this.copy,
    required this.onAuthorityLost,
  });

  final Revision3NpcGreetingProjection projection;
  final Revision3NpcGreetingAuthoringService service;
  final ValueListenable<String?> mutationBlock;
  final Revision3NpcDialogVoicePanelCopy copy;
  final VoidCallback onAuthorityLost;

  @override
  State<_Revision3NpcGreetingReviewDialog> createState() =>
      _Revision3NpcGreetingReviewDialogState();
}

class _Revision3NpcGreetingReviewDialogState
    extends State<_Revision3NpcGreetingReviewDialog> {
  late final Revision3NpcGreetingDraft _draft =
      Revision3NpcGreetingDraft.fromProjection(widget.projection);
  bool _saving = false;
  bool _allowPop = false;
  bool _confirmingDiscard = false;
  bool _authorityLost = false;
  String? _error;
  int _saveEpoch = 0;

  bool get _hasChanges {
    final rows = _draft.rows;
    final original = widget.projection.rows;
    if (rows.length != original.length) return true;
    for (var index = 0; index < rows.length; index++) {
      if (rows[index].line.lineId != original[index].lineId) return true;
    }
    return false;
  }

  @override
  void dispose() {
    _saveEpoch++;
    super.dispose();
  }

  void _attach(Revision3NpcGreetingLineChoice? choice, String? block) {
    if (choice == null || block != null || _saving || _authorityLost) return;
    setState(() {
      _draft.attach(choice);
      _error = null;
    });
  }

  void _move(int index, int target, String? block) {
    if (block != null || _saving || _authorityLost) return;
    setState(() {
      _draft.reorder(fromIndex: index, toIndex: target);
      _error = null;
    });
  }

  void _detach(int index, String? block) {
    if (block != null || _saving || _authorityLost) return;
    setState(() {
      _draft.detachAt(index);
      _error = null;
    });
  }

  Future<void> _save(String? block) async {
    if (block != null || _saving || _authorityLost || !_hasChanges) return;
    final epoch = ++_saveEpoch;
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final publication = await widget.service.replace(
        projection: widget.projection,
        draft: _draft,
      );
      if (!mounted || epoch != _saveEpoch) return;
      await _popAfterUnlock(publication);
    } catch (error) {
      if (!mounted || epoch != _saveEpoch) return;
      final authorityLost = _npcGreetingAuthorityError(error);
      setState(() {
        _saving = false;
        _authorityLost = authorityLost;
        _error = authorityLost
            ? widget.copy.requiresReopen
            : widget.copy.saveFailed;
      });
      if (authorityLost) widget.onAuthorityLost();
    }
  }

  Future<void> _requestDismiss() async {
    if (_saving || _confirmingDiscard) return;
    if (!_hasChanges) {
      await _popAfterUnlock();
      return;
    }
    setState(() => _confirmingDiscard = true);
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const Key('revision3-npc-greeting-discard-dialog'),
        scrollable: true,
        title: Text(widget.copy.discardTitle),
        content: Text(widget.copy.discardDescription),
        actions: [
          TextButton(
            key: const Key('revision3-npc-greeting-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(widget.copy.keepEditing),
          ),
          FilledButton(
            key: const Key('revision3-npc-greeting-discard'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(widget.copy.discard),
          ),
        ],
      ),
    );
    if (!mounted) return;
    setState(() => _confirmingDiscard = false);
    if (discard == true) await _popAfterUnlock();
  }

  Future<void> _popAfterUnlock([
    Revision3NpcGreetingPublication? publication,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(publication);
  }

  @override
  Widget build(BuildContext context) => ValueListenableBuilder<String?>(
    valueListenable: widget.mutationBlock,
    builder: (context, externalBlock, _) {
      final block = _authorityLost ? widget.copy.requiresReopen : externalBlock;
      final size = MediaQuery.sizeOf(context);
      final compact =
          size.width < 600 || MediaQuery.textScalerOf(context).scale(16) >= 24;
      final dialog = compact
          ? _buildCompactDialog(context, block)
          : _buildWideDialog(context, block);
      return PopScope<Revision3NpcGreetingPublication?>(
        canPop: _allowPop || (!_saving && !_hasChanges),
        onPopInvokedWithResult: (didPop, _) {
          if (!didPop) unawaited(_requestDismiss());
        },
        child: dialog,
      );
    },
  );

  Widget _buildWideDialog(BuildContext context, String? block) => AlertDialog(
    key: const Key('revision3-npc-greeting-review-dialog'),
    title: Text(widget.copy.reviewTitle),
    content: SizedBox(
      width: 680,
      height: MediaQuery.sizeOf(context).height.clamp(280.0, 620.0) * 0.72,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildReviewIntroduction(block),
          const SizedBox(height: 12),
          _buildAttach(block),
          const SizedBox(height: 8),
          Expanded(child: _buildRows(block)),
        ],
      ),
    ),
    actions: [_buildCancelButton(), _buildSaveButton(block)],
  );

  Widget _buildCompactDialog(BuildContext context, String? block) {
    final size = MediaQuery.sizeOf(context);
    return Dialog.fullscreen(
      key: const Key('revision3-npc-greeting-review-dialog'),
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: size.height * 0.44),
              child: SingleChildScrollView(
                key: const Key('revision3-npc-greeting-review-chrome-scroll'),
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 10),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(
                      widget.copy.reviewTitle,
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 10),
                    _buildReviewIntroduction(block),
                    const SizedBox(height: 12),
                    _buildAttach(block),
                  ],
                ),
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: _buildRows(block),
              ),
            ),
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Wrap(
                alignment: WrapAlignment.end,
                spacing: 8,
                runSpacing: 8,
                children: [_buildCancelButton(), _buildSaveButton(block)],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildReviewIntroduction(String? block) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Text(widget.copy.reviewDescription),
      if (block != null) ...[
        const SizedBox(height: 8),
        _NpcGreetingNotice(icon: Icons.lock_outline, text: block),
      ],
      if (_error != null) ...[
        const SizedBox(height: 8),
        _NpcGreetingNotice(icon: Icons.error_outline, text: _error!),
      ],
    ],
  );

  Widget _buildCancelButton() => TextButton(
    key: const Key('revision3-npc-greeting-review-cancel'),
    onPressed: _saving || _confirmingDiscard ? null : _requestDismiss,
    child: Text(widget.copy.cancel),
  );

  Widget _buildSaveButton(String? block) => FilledButton.icon(
    key: const Key('revision3-npc-greeting-review-save'),
    onPressed: block != null || _saving || !_hasChanges
        ? null
        : () => _save(block),
    icon: _saving
        ? const SizedBox.square(
            dimension: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : const Icon(Icons.save_outlined),
    label: Text(_saving ? widget.copy.saving : widget.copy.save),
  );

  Widget _buildAttach(String? block) {
    final choices = _draft.unboundChoices;
    if (choices.isEmpty) {
      return Text(
        widget.copy.noUnboundLines,
        key: const Key('revision3-npc-greeting-no-unbound'),
      );
    }
    return DropdownButtonFormField<Revision3NpcGreetingLineChoice>(
      key: ValueKey('revision3-npc-greeting-attach-${choices.length}'),
      initialValue: null,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: widget.copy.attachExisting,
        prefixIcon: const Icon(Icons.link_outlined),
        isDense: true,
      ),
      items: [
        for (final choice in choices)
          DropdownMenuItem(
            value: choice,
            child: Text(
              choice.displayLabel,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
      ],
      onChanged: block != null || _saving
          ? null
          : (choice) => _attach(choice, block),
    );
  }

  Widget _buildRows(String? block) {
    final rows = _draft.rows;
    if (rows.isEmpty) return Center(child: Text(widget.copy.emptyTitle));
    return ListView.separated(
      key: const Key('revision3-npc-greeting-review-rows'),
      itemCount: rows.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final row = rows[index];
        return Padding(
          key: Key('revision3-npc-greeting-review-row-$index'),
          padding: const EdgeInsets.symmetric(vertical: 6),
          child: Row(
            children: [
              SizedBox(width: 30, child: Text('${index + 1}')),
              Expanded(
                child: Text(
                  row.line.displayLabel,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              IconButton(
                key: Key('revision3-npc-greeting-review-up-$index'),
                tooltip: widget.copy.moveUp,
                onPressed: block != null || _saving || index == 0
                    ? null
                    : () => _move(index, index - 1, block),
                icon: const Icon(Icons.arrow_upward),
              ),
              IconButton(
                key: Key('revision3-npc-greeting-review-down-$index'),
                tooltip: widget.copy.moveDown,
                onPressed: block != null || _saving || index == rows.length - 1
                    ? null
                    : () => _move(index, index + 1, block),
                icon: const Icon(Icons.arrow_downward),
              ),
              IconButton(
                key: Key('revision3-npc-greeting-review-detach-$index'),
                tooltip: widget.copy.removeFromGreetings,
                onPressed: block != null || _saving
                    ? null
                    : () => _detach(index, block),
                icon: const Icon(Icons.link_off_outlined),
              ),
            ],
          ),
        );
      },
    );
  }
}
