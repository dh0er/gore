import 'dart:async';

import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';

import 'revision3_dialog_line_authoring.dart';
import 'revision3_quest_transcript_authoring.dart';

typedef Revision3QuestTranscriptCreateLineAction =
    Future<bool> Function({
      required Revision3QuestTranscriptProjection projection,
      required int insertionIndex,
      required int? objectiveSlot,
      required Revision3DialogLineEntryTechnicalPublisher publishTechnicalPlan,
    });

typedef Revision3QuestTranscriptOpenTextVoiceAction =
    Future<bool> Function({
      required Revision3QuestTranscriptProjection projection,
      required Revision3QuestTranscriptRow row,
      required String locale,
    });

typedef Revision3QuestTranscriptPublishedAction =
    Future<void> Function(Revision3QuestTranscriptPublication publication);

@immutable
final class Revision3QuestTranscriptPanelCopy {
  const Revision3QuestTranscriptPanelCopy({
    required this.title,
    required this.description,
    required this.loading,
    required this.loadFailedTitle,
    required this.loadFailedDescription,
    required this.retry,
    required this.newLine,
    required this.editTranscript,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.ungrouped,
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
    required this.objectiveLabel,
    required this.moveUp,
    required this.moveDown,
    required this.removeFromTranscript,
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

  static const english = Revision3QuestTranscriptPanelCopy(
    title: 'Quest transcript',
    description:
        'Arrange project dialog lines, group them by objective, and review text and Voice coverage.',
    loading: 'Opening the exact Quest transcript\u2026',
    loadFailedTitle: 'Quest transcript unavailable',
    loadFailedDescription:
        'The exact current transcript could not be verified. Refresh the project before editing.',
    retry: 'Retry',
    newLine: 'New line',
    editTranscript: 'Edit transcript',
    emptyTitle: 'No dialog lines yet',
    emptyDescription:
        'Create the first line or attach an existing unbound project line.',
    ungrouped: 'General dialog',
    lineCountTemplate: '{count} lines',
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
    reviewTitle: 'Edit Quest transcript',
    reviewDescription:
        'Attach existing project lines, change their order or objective group, or remove them from this transcript.',
    attachExisting: 'Attach existing line',
    noUnboundLines: 'No unbound project lines are available.',
    objectiveLabel: 'Objective group',
    moveUp: 'Move up',
    moveDown: 'Move down',
    removeFromTranscript: 'Remove from transcript',
    cancel: 'Cancel',
    save: 'Save transcript',
    saving: 'Saving\u2026',
    discardTitle: 'Discard transcript changes?',
    discardDescription:
        'The unsaved order, objective groups, attachments, and removals will be lost.',
    keepEditing: 'Keep editing',
    discard: 'Discard changes',
    mutationDisabledFallback: 'Transcript editing is currently unavailable.',
    requiresReopen:
        'The project changed or exact authority was lost. Refresh or reopen the project before editing.',
    waitingForRefresh: 'Published. Waiting for the refreshed project\u2026',
    saveFailed: 'The transcript could not be saved.',
    createFailed: 'The new line could not be created.',
    maximumReached: 'This Quest already contains the maximum of 256 lines.',
  );

  static const german = Revision3QuestTranscriptPanelCopy(
    title: 'Quest-Transkript',
    description:
        'Projekt-Dialogzeilen anordnen, Quest-Zielen zuordnen und Text- sowie Voice-Abdeckung pr\u00fcfen.',
    loading: 'Das exakte Quest-Transkript wird ge\u00f6ffnet\u2026',
    loadFailedTitle: 'Quest-Transkript nicht verf\u00fcgbar',
    loadFailedDescription:
        'Das aktuelle Transkript konnte nicht eindeutig gepr\u00fcft werden. Aktualisiere das Projekt vor dem Bearbeiten.',
    retry: 'Erneut versuchen',
    newLine: 'Neue Zeile',
    editTranscript: 'Transkript bearbeiten',
    emptyTitle: 'Noch keine Dialogzeilen',
    emptyDescription:
        'Erstelle die erste Zeile oder verkn\u00fcpfe eine vorhandene, noch ungebundene Projektzeile.',
    ungrouped: 'Allgemeiner Dialog',
    lineCountTemplate: '{count} Zeilen',
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
    reviewTitle: 'Quest-Transkript bearbeiten',
    reviewDescription:
        'Vorhandene Projektzeilen verkn\u00fcpfen, Reihenfolge oder Zielgruppe \u00e4ndern oder Zeilen aus diesem Transkript entfernen.',
    attachExisting: 'Vorhandene Zeile verkn\u00fcpfen',
    noUnboundLines: 'Es sind keine ungebundenen Projektzeilen verf\u00fcgbar.',
    objectiveLabel: 'Quest-Zielgruppe',
    moveUp: 'Nach oben',
    moveDown: 'Nach unten',
    removeFromTranscript: 'Aus Transkript entfernen',
    cancel: 'Abbrechen',
    save: 'Transkript speichern',
    saving: 'Wird gespeichert\u2026',
    discardTitle: 'Transkript\u00e4nderungen verwerfen?',
    discardDescription:
        'Ungespeicherte Reihenfolge, Zielgruppen, Verkn\u00fcpfungen und Entfernungen gehen verloren.',
    keepEditing: 'Weiter bearbeiten',
    discard: '\u00c4nderungen verwerfen',
    mutationDisabledFallback:
        'Das Quest-Transkript kann momentan nicht bearbeitet werden.',
    requiresReopen:
        'Das Projekt hat sich ge\u00e4ndert oder die exakte Berechtigung ging verloren. Aktualisiere oder \u00f6ffne es erneut.',
    waitingForRefresh:
        'Ver\u00f6ffentlicht. Das aktualisierte Projekt wird erwartet\u2026',
    saveFailed: 'Das Transkript konnte nicht gespeichert werden.',
    createFailed: 'Die neue Dialogzeile konnte nicht erstellt werden.',
    maximumReached:
        'Diese Quest enth\u00e4lt bereits das Maximum von 256 Zeilen.',
  );

  final String title;
  final String description;
  final String loading;
  final String loadFailedTitle;
  final String loadFailedDescription;
  final String retry;
  final String newLine;
  final String editTranscript;
  final String emptyTitle;
  final String emptyDescription;
  final String ungrouped;
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
  final String objectiveLabel;
  final String moveUp;
  final String moveDown;
  final String removeFromTranscript;
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

/// Responsive, presentation-only Quest transcript workbench.
///
/// All technical identities remain in the exact projection and are passed only
/// to host callbacks. This widget never renders IDs, LocIDs, hashes, or paths.
final class Revision3QuestTranscriptPanel extends StatefulWidget {
  const Revision3QuestTranscriptPanel({
    required this.projectId,
    required this.projectRevision,
    required this.projectCheckpointIdentity,
    required this.questId,
    required this.questRevision,
    required this.service,
    required this.selectedLineId,
    required this.onSelectedLineChanged,
    required this.onCreateLine,
    required this.onOpenTextVoice,
    this.onPublished,
    this.mutationsEnabled = true,
    this.mutationDisabledReason,
    this.copy = Revision3QuestTranscriptPanelCopy.english,
    super.key,
  }) : assert(projectId != ''),
       assert(projectRevision >= 1),
       assert(questId != ''),
       assert(questRevision >= 1),
       assert(
         mutationsEnabled ||
             (mutationDisabledReason != null && mutationDisabledReason != ''),
       );

  final String projectId;
  final int projectRevision;
  final Object projectCheckpointIdentity;
  final String questId;
  final int questRevision;
  final Revision3QuestTranscriptAuthoringService service;
  final String? selectedLineId;
  final ValueChanged<String?> onSelectedLineChanged;
  final Revision3QuestTranscriptCreateLineAction onCreateLine;
  final Revision3QuestTranscriptOpenTextVoiceAction onOpenTextVoice;
  final Revision3QuestTranscriptPublishedAction? onPublished;
  final bool mutationsEnabled;
  final String? mutationDisabledReason;
  final Revision3QuestTranscriptPanelCopy copy;

  @override
  State<Revision3QuestTranscriptPanel> createState() =>
      _Revision3QuestTranscriptPanelState();
}

class _Revision3QuestTranscriptPanelState
    extends State<Revision3QuestTranscriptPanel> {
  Revision3QuestTranscriptProjection? _projection;
  Revision3QuestTranscriptTextPreview? _preview;
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
  Route<Revision3QuestTranscriptPublication?>? _reviewRoute;

  @override
  void initState() {
    super.initState();
    _selectedLineId = widget.selectedLineId;
    _reviewMutationBlock = ValueNotifier<String?>(_mutationBlockReason);
    unawaited(_load());
  }

  @override
  void didUpdateWidget(covariant Revision3QuestTranscriptPanel oldWidget) {
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
        oldWidget.questId != widget.questId ||
        oldWidget.questRevision != widget.questRevision;
    if (checkpointChanged) {
      final reviewRoute = _reviewRoute;
      if (reviewRoute != null && reviewRoute.isActive) {
        reviewRoute.navigator?.removeRoute(reviewRoute);
      }
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
    _reviewRoute = null;
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
    final reason = _mutationBlockReason;
    if (_reviewMutationBlock.value != reason) {
      _reviewMutationBlock.value = reason;
    }
  }

  bool _matchesWidget(Revision3QuestTranscriptProjection projection) =>
      projection.projectId == widget.projectId &&
      projection.projectRevision == widget.projectRevision &&
      projection.checkpointIdentity == widget.projectCheckpointIdentity &&
      projection.questId == widget.questId &&
      projection.questRevision == widget.questRevision;

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
        questId: widget.questId,
        expectedQuestRevision: widget.questRevision,
      );
      if (!mounted || epoch != _loadEpoch) return;
      if (!_matchesWidget(projection)) {
        throw const Revision3QuestTranscriptStaleCheckpointException();
      }
      setState(() {
        _projection = projection;
        _loading = false;
        _loadError = null;
        _requiresReopen = false;
      });
      _syncReviewMutationBlock();
      _reconcileSelection(loadPreview: true);
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() {
        _loading = false;
        _loadError = error;
        _requiresReopen = _isAuthorityError(error);
      });
      _syncReviewMutationBlock();
    }
  }

  Revision3QuestTranscriptRow? get _selectedRow {
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
    final locales = row == null ? const <String>[] : _rowLocales(row);
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

  Future<void> _loadPreview(Revision3QuestTranscriptRow row) async {
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
        if (_isAuthorityError(error)) _requiresReopen = true;
      });
      _syncReviewMutationBlock();
    }
  }

  void _selectRow(Revision3QuestTranscriptRow row) {
    if (identical(_selectedRow, row)) return;
    _selectedLineId = row.lineId;
    widget.onSelectedLineChanged(row.lineId);
    final locales = _rowLocales(row);
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
    final objectiveSlot = selected?.objectiveSlot;
    final epoch = ++_actionEpoch;
    setState(() {
      _busy = true;
      _actionMessage = null;
    });
    try {
      final exactPublisher = widget.service.createAndInsertPublisher(
        projection: projection,
        index: insertionIndex,
        objectiveSlot: objectiveSlot,
      );
      final published = await widget.onCreateLine(
        projection: projection,
        insertionIndex: insertionIndex,
        objectiveSlot: objectiveSlot,
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
                throw const Revision3QuestTranscriptStaleCheckpointException();
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
        _requiresReopen = _isAuthorityError(error);
        _actionMessage = widget.copy.createFailed;
      });
      _syncReviewMutationBlock();
    }
  }

  Future<void> _editTranscript() async {
    final projection = _projection;
    if (projection == null || _busy || _mutationBlockReason != null) return;
    Route<Revision3QuestTranscriptPublication?>? route;
    Revision3QuestTranscriptPublication? publication;
    try {
      publication = await showDialog<Revision3QuestTranscriptPublication>(
        context: context,
        barrierDismissible: false,
        builder: (dialogContext) {
          route ??=
              ModalRoute.of(dialogContext)
                  as Route<Revision3QuestTranscriptPublication?>?;
          _reviewRoute = route;
          return _Revision3QuestTranscriptReviewDialog(
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
    final panelHeight = (media.height * (media.width < 720 ? 0.82 : 0.62))
        .clamp(media.width < 720 ? 360.0 : 300.0, 680.0);
    return Material(
      key: const Key('revision3-quest-transcript-panel'),
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
            _buildHeader(context, projection, mutationBlock, maxReached),
            if (mutationBlock != null)
              _TranscriptNotice(
                key: const Key('revision3-quest-transcript-mutation-block'),
                icon: Icons.lock_outline,
                text: mutationBlock,
              ),
            if (_actionMessage != null)
              _TranscriptNotice(
                key: const Key('revision3-quest-transcript-action-message'),
                icon: Icons.info_outline,
                text: _actionMessage!,
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
    Revision3QuestTranscriptProjection? projection,
    String? mutationBlock,
    bool maxReached,
  ) {
    final copy = widget.copy;
    return Padding(
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
                    copy.title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                const SizedBox(height: 2),
                Tooltip(
                  message: MediaQuery.sizeOf(context).width < 520
                      ? copy.description
                      : '',
                  child: Text(
                    copy.description,
                    maxLines: MediaQuery.sizeOf(context).width < 520 ? 2 : null,
                    overflow: MediaQuery.sizeOf(context).width < 520
                        ? TextOverflow.ellipsis
                        : null,
                  ),
                ),
                if (projection != null)
                  Text(
                    copy.lineCount(projection.rows.length),
                    key: const Key('revision3-quest-transcript-line-count'),
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
                message: maxReached ? copy.maximumReached : mutationBlock ?? '',
                child: FilledButton.icon(
                  key: const Key('revision3-quest-transcript-new-line'),
                  onPressed:
                      projection == null ||
                          _busy ||
                          mutationBlock != null ||
                          maxReached
                      ? null
                      : _createLine,
                  icon: _busy
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.add_comment_outlined),
                  label: Text(
                    copy.newLine,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
              Tooltip(
                message: mutationBlock ?? '',
                child: OutlinedButton.icon(
                  key: const Key('revision3-quest-transcript-edit'),
                  onPressed:
                      projection == null || _busy || mutationBlock != null
                      ? null
                      : _editTranscript,
                  icon: const Icon(Icons.reorder_outlined),
                  label: Text(
                    copy.editTranscript,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildBody(BuildContext context) {
    if (_loading) {
      return Center(
        child: Semantics(
          liveRegion: true,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 10),
              Text(widget.copy.loading),
            ],
          ),
        ),
      );
    }
    if (_loadError != null || _projection == null) {
      return _TranscriptLoadError(
        copy: widget.copy,
        retry: _requiresReopen ? null : _load,
      );
    }
    final projection = _projection!;
    if (projection.rows.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.forum_outlined, size: 36),
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
        final wide = constraints.maxWidth >= 720;
        final list = _TranscriptLineList(
          projection: projection,
          selected: _selectedRow,
          copy: widget.copy,
          onSelected: _selectRow,
        );
        final detail = _buildSelectedDetail(context);
        if (wide) {
          return Row(
            key: const Key('revision3-quest-transcript-wide'),
            children: [
              Expanded(flex: 5, child: list),
              const VerticalDivider(width: 1),
              Expanded(flex: 6, child: detail),
            ],
          );
        }
        return Column(
          key: const Key('revision3-quest-transcript-compact'),
          children: [
            SizedBox(
              height: (constraints.maxHeight * 0.45).clamp(60.0, 190.0),
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
    final locales = _rowLocales(row);
    final selectedLocale = locales.contains(_selectedLocale)
        ? _selectedLocale
        : locales.firstOrNull;
    final previewLocale = _preview?.locales
        .where((locale) => locale.locale == selectedLocale)
        .firstOrNull;
    return SingleChildScrollView(
      key: const Key('revision3-quest-transcript-detail'),
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
                key: const Key('revision3-quest-transcript-locale'),
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
            key: const Key('revision3-quest-transcript-preview'),
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
              key: const Key('revision3-quest-transcript-open-text-voice'),
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

List<String> _rowLocales(Revision3QuestTranscriptRow row) {
  final locales = <String>{
    ...row.authoredLocales,
    for (final coverage in row.localeCoverage) coverage.locale,
  }.toList(growable: false)..sort();
  return locales;
}

bool _isAuthorityError(Object error) =>
    error is Revision3QuestTranscriptRequiresReopenException ||
    error is Revision3QuestTranscriptStaleCheckpointException;

class _TranscriptLineList extends StatelessWidget {
  const _TranscriptLineList({
    required this.projection,
    required this.selected,
    required this.copy,
    required this.onSelected,
  });

  final Revision3QuestTranscriptProjection projection;
  final Revision3QuestTranscriptRow? selected;
  final Revision3QuestTranscriptPanelCopy copy;
  final ValueChanged<Revision3QuestTranscriptRow> onSelected;

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      key: const Key('revision3-quest-transcript-lines'),
      padding: const EdgeInsets.symmetric(vertical: 6),
      itemCount: projection.rows.length,
      itemBuilder: (context, index) {
        final row = projection.rows[index];
        final previousSlot = index == 0
            ? const _NoPreviousObjective()
            : projection.rows[index - 1].objectiveSlot;
        final beginsGroup =
            previousSlot is _NoPreviousObjective ||
            previousSlot != row.objectiveSlot;
        final objective = row.objectiveSlot == null
            ? null
            : projection.objectiveBySlot(row.objectiveSlot!);
        final locales = _rowLocales(row);
        final localeCount = locales.length;
        final authoredCount = row.localeCoverage
            .where((locale) => locale.hasAuthoredText)
            .length;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (beginsGroup)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 10, 16, 4),
                child: Text(
                  objective?.title ?? copy.ungrouped,
                  key: Key('revision3-quest-transcript-group-$index'),
                  style: Theme.of(context).textTheme.labelLarge,
                ),
              ),
            Semantics(
              button: true,
              selected: identical(selected, row),
              child: ListTile(
                key: Key('revision3-quest-transcript-row-$index'),
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
                        _CoverageChip(
                          label: locales.isEmpty
                              ? copy.noLocales
                              : copy.locales(locales),
                          icon: Icons.language_outlined,
                        ),
                        _CoverageChip(
                          label: copy.textCoverage(authoredCount, localeCount),
                          icon: Icons.translate_outlined,
                        ),
                        _CoverageChip(
                          label: copy.voiceCoverage(
                            row.voiceSlotCount,
                            localeCount,
                            row.voiceTakeCount,
                          ),
                          icon: Icons.mic_none_outlined,
                        ),
                        if (row.selectedVoiceTakeCount > 0)
                          _CoverageChip(
                            label: copy.selectedVoice(
                              row.selectedVoiceTakeCount,
                            ),
                            icon: Icons.check_circle_outline,
                          ),
                      ],
                    ),
                  ],
                ),
                onTap: () => onSelected(row),
              ),
            ),
          ],
        );
      },
    );
  }
}

final class _NoPreviousObjective {
  const _NoPreviousObjective();
}

class _CoverageChip extends StatelessWidget {
  const _CoverageChip({required this.label, required this.icon});

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

class _TranscriptNotice extends StatelessWidget {
  const _TranscriptNotice({required this.icon, required this.text, super.key});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      color: colors.secondaryContainer,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 18, color: colors.onSecondaryContainer),
          const SizedBox(width: 8),
          Expanded(
            child: Tooltip(
              message: text,
              child: Text(
                text,
                style: TextStyle(color: colors.onSecondaryContainer),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _TranscriptLoadError extends StatelessWidget {
  const _TranscriptLoadError({required this.copy, required this.retry});

  final Revision3QuestTranscriptPanelCopy copy;
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
              key: const Key('revision3-quest-transcript-retry'),
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

class _Revision3QuestTranscriptReviewDialog extends StatefulWidget {
  const _Revision3QuestTranscriptReviewDialog({
    required this.projection,
    required this.service,
    required this.mutationBlock,
    required this.copy,
    required this.onAuthorityLost,
  });

  final Revision3QuestTranscriptProjection projection;
  final Revision3QuestTranscriptAuthoringService service;
  final ValueListenable<String?> mutationBlock;
  final Revision3QuestTranscriptPanelCopy copy;
  final VoidCallback onAuthorityLost;

  @override
  State<_Revision3QuestTranscriptReviewDialog> createState() =>
      _Revision3QuestTranscriptReviewDialogState();
}

class _Revision3QuestTranscriptReviewDialogState
    extends State<_Revision3QuestTranscriptReviewDialog> {
  late final Revision3QuestTranscriptDraft _draft =
      Revision3QuestTranscriptDraft.fromProjection(widget.projection);
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
      if (rows[index].line.lineId != original[index].lineId ||
          rows[index].objectiveSlot != original[index].objectiveSlot) {
        return true;
      }
    }
    return false;
  }

  @override
  void dispose() {
    _saveEpoch++;
    super.dispose();
  }

  void _attach(Revision3QuestTranscriptLineChoice? choice, String? block) {
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

  void _setObjective(int index, int selection, String? block) {
    if (block != null || _saving || _authorityLost) return;
    setState(() {
      _draft.setObjectiveSlot(
        index: index,
        objectiveSlot: selection == -1 ? null : selection,
      );
      _error = null;
    });
  }

  Future<void> _save(String? block) async {
    if (block != null || _saving || _authorityLost || !_hasChanges) {
      return;
    }
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
      final authorityLost = _isAuthorityError(error);
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
        key: const Key('revision3-quest-transcript-discard-dialog'),
        title: Text(widget.copy.discardTitle),
        content: Text(widget.copy.discardDescription),
        actions: [
          TextButton(
            key: const Key('revision3-quest-transcript-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(widget.copy.keepEditing),
          ),
          FilledButton(
            key: const Key('revision3-quest-transcript-discard'),
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
    Revision3QuestTranscriptPublication? publication,
  ]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(publication);
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<String?>(
      valueListenable: widget.mutationBlock,
      builder: (context, externalBlock, _) {
        final block = _authorityLost
            ? widget.copy.requiresReopen
            : externalBlock;
        final dialog = AlertDialog(
          key: const Key('revision3-quest-transcript-review-dialog'),
          title: Text(widget.copy.reviewTitle),
          content: SizedBox(
            width: 720,
            height:
                MediaQuery.sizeOf(context).height.clamp(280.0, 620.0) * 0.72,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(widget.copy.reviewDescription),
                if (block != null) ...[
                  const SizedBox(height: 8),
                  _TranscriptNotice(icon: Icons.lock_outline, text: block),
                ],
                if (_error != null) ...[
                  const SizedBox(height: 8),
                  _TranscriptNotice(icon: Icons.error_outline, text: _error!),
                ],
                const SizedBox(height: 12),
                _buildAttach(context, block),
                const SizedBox(height: 8),
                Expanded(child: _buildRows(context, block)),
              ],
            ),
          ),
          actions: [
            TextButton(
              key: const Key('revision3-quest-transcript-review-cancel'),
              onPressed: _saving || _confirmingDiscard ? null : _requestDismiss,
              child: Text(widget.copy.cancel),
            ),
            FilledButton.icon(
              key: const Key('revision3-quest-transcript-review-save'),
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
            ),
          ],
        );
        return PopScope<Revision3QuestTranscriptPublication?>(
          canPop: _allowPop || (!_saving && !_hasChanges),
          onPopInvokedWithResult: (didPop, _) {
            if (!didPop) unawaited(_requestDismiss());
          },
          child: dialog,
        );
      },
    );
  }

  Widget _buildAttach(BuildContext context, String? block) {
    final choices = _draft.unboundChoices;
    if (choices.isEmpty) {
      return Text(
        widget.copy.noUnboundLines,
        key: const Key('revision3-quest-transcript-no-unbound'),
      );
    }
    return DropdownButtonFormField<Revision3QuestTranscriptLineChoice>(
      key: ValueKey('revision3-quest-transcript-attach-${choices.length}'),
      initialValue: null,
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

  Widget _buildRows(BuildContext context, String? block) {
    final rows = _draft.rows;
    if (rows.isEmpty) {
      return Center(child: Text(widget.copy.emptyTitle));
    }
    return ListView.separated(
      key: const Key('revision3-quest-transcript-review-rows'),
      itemCount: rows.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final row = rows[index];
        final selectedObjective = row.objectiveSlot ?? -1;
        return Padding(
          key: Key('revision3-quest-transcript-review-row-$index'),
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.only(top: 10),
                child: Text('${index + 1}'),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(
                      row.line.displayLabel,
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    const SizedBox(height: 6),
                    DropdownButtonFormField<int>(
                      key: Key(
                        'revision3-quest-transcript-review-objective-$index',
                      ),
                      initialValue: selectedObjective,
                      decoration: InputDecoration(
                        labelText: widget.copy.objectiveLabel,
                        isDense: true,
                      ),
                      items: [
                        DropdownMenuItem(
                          value: -1,
                          child: Text(widget.copy.ungrouped),
                        ),
                        for (final objective in widget.projection.objectives)
                          DropdownMenuItem(
                            value: objective.slot,
                            child: Text(
                              objective.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                      ],
                      onChanged: block != null || _saving
                          ? null
                          : (value) {
                              if (value != null) {
                                _setObjective(index, value, block);
                              }
                            },
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 6),
              Column(
                children: [
                  IconButton(
                    key: Key('revision3-quest-transcript-review-up-$index'),
                    tooltip: widget.copy.moveUp,
                    onPressed: block != null || _saving || index == 0
                        ? null
                        : () => _move(index, index - 1, block),
                    icon: const Icon(Icons.arrow_upward),
                  ),
                  IconButton(
                    key: Key('revision3-quest-transcript-review-down-$index'),
                    tooltip: widget.copy.moveDown,
                    onPressed:
                        block != null || _saving || index == rows.length - 1
                        ? null
                        : () => _move(index, index + 1, block),
                    icon: const Icon(Icons.arrow_downward),
                  ),
                  IconButton(
                    key: Key('revision3-quest-transcript-review-detach-$index'),
                    tooltip: widget.copy.removeFromTranscript,
                    onPressed: block != null || _saving
                        ? null
                        : () => _detach(index, block),
                    icon: const Icon(Icons.link_off_outlined),
                  ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }
}
