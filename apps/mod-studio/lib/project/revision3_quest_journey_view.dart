import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_content_index.dart';
import 'revision3_quest_journey.dart';
import 'revision3_quest_journey_panel.dart';
import 'revision3_quest_journey_service.dart';

/// Exact-checkpoint lifecycle owner for [Revision3QuestJourneyPanel].
///
/// This widget performs only the service's read-only project load. It owns no
/// writer, game root, build/deployment operation, runtime authority, or save
/// access. Editor actions remain explicit owner-supplied panel callbacks.
class Revision3QuestJourneyView extends StatefulWidget {
  const Revision3QuestJourneyView({
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.index,
    required this.quest,
    required this.service,
    this.authorityEpoch = 0,
    this.giverDisplayName,
    this.parentStoryDisplayName,
    this.onEditNameObjectives,
    this.onEditDescriptionConnections,
    this.onEditStatesTransitions,
    this.onOpenDialogVoice,
    this.editDisabledReason,
    this.editNameObjectivesDisabledReason,
    this.editDescriptionConnectionsDisabledReason,
    this.editStatesTransitionsDisabledReason,
    this.openDialogVoiceDisabledReason,
    this.onOpenDialogLine,
    this.copy = const Revision3QuestJourneyPanelCopy.english(),
    super.key,
  }) : assert(authorityEpoch >= 0);

  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final Revision3QuestJourneyService service;

  /// Owner-controlled generation of the read authority behind [service].
  ///
  /// Increment this only after a verified recovery or reopen establishes fresh
  /// authority for the same visible checkpoint. Service identity is
  /// deliberately not a load key because owners may rebuild equivalent service
  /// facades without changing project authority.
  final int authorityEpoch;
  final String? giverDisplayName;
  final String? parentStoryDisplayName;
  final Revision3QuestJourneyAction? onEditNameObjectives;
  final Revision3QuestJourneyAction? onEditDescriptionConnections;
  final Revision3QuestJourneyAction? onEditStatesTransitions;
  final Revision3QuestJourneyAction? onOpenDialogVoice;

  /// Localized owner-provided reason that keeps all edit affordances visible
  /// but disabled. Leave null together with null callbacks for a deliberately
  /// read-only journey.
  final String? editDisabledReason;

  /// Localized reasons that disable only their matching edit action. The
  /// shared [editDisabledReason] takes precedence when both are supplied.
  final String? editNameObjectivesDisabledReason;
  final String? editDescriptionConnectionsDisabledReason;
  final String? editStatesTransitionsDisabledReason;
  final String? openDialogVoiceDisabledReason;
  final Revision3QuestJourneyOpenDialogLine? onOpenDialogLine;
  final Revision3QuestJourneyPanelCopy copy;

  @override
  State<Revision3QuestJourneyView> createState() =>
      _Revision3QuestJourneyViewState();
}

enum _Revision3QuestJourneyViewPhase {
  loading,
  available,
  retryableUnavailable,
  requiresReopen,
}

final class _Revision3QuestJourneyViewState
    extends State<Revision3QuestJourneyView> {
  _Revision3QuestJourneyViewPhase _phase =
      _Revision3QuestJourneyViewPhase.loading;
  Revision3QuestJourneyProjection? _projection;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_beginLoad(notify: false));
  }

  @override
  void didUpdateWidget(covariant Revision3QuestJourneyView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.checkpointIdentity != widget.checkpointIdentity ||
        !identical(oldWidget.index, widget.index) ||
        !identical(oldWidget.quest, widget.quest) ||
        oldWidget.authorityEpoch != widget.authorityEpoch) {
      unawaited(_beginLoad(notify: false));
    }
  }

  @override
  void dispose() {
    _loadGeneration++;
    super.dispose();
  }

  Future<void> _beginLoad({required bool notify}) {
    final generation = ++_loadGeneration;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final checkpointIdentity = widget.checkpointIdentity;
    final index = widget.index;
    final quest = widget.quest;
    final service = widget.service;
    final authorityEpoch = widget.authorityEpoch;
    void begin() {
      _phase = _Revision3QuestJourneyViewPhase.loading;
      _projection = null;
    }

    if (notify) {
      setState(begin);
    } else {
      begin();
    }
    return _load(
      generation: generation,
      projectId: projectId,
      projectRevision: projectRevision,
      checkpointIdentity: checkpointIdentity,
      index: index,
      quest: quest,
      service: service,
      authorityEpoch: authorityEpoch,
    );
  }

  Future<void> _load({
    required int generation,
    required String projectId,
    required int projectRevision,
    required String checkpointIdentity,
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required Revision3QuestJourneyService service,
    required int authorityEpoch,
  }) async {
    bool requestIsCurrent() =>
        mounted &&
        generation == _loadGeneration &&
        widget.projectId == projectId &&
        widget.projectRevision == projectRevision &&
        widget.checkpointIdentity == checkpointIdentity &&
        identical(widget.index, index) &&
        identical(widget.quest, quest) &&
        widget.authorityEpoch == authorityEpoch;

    try {
      _requireExactInput(
        projectId: projectId,
        projectRevision: projectRevision,
        checkpointIdentity: checkpointIdentity,
        index: index,
        quest: quest,
      );
      final projection = await service.load(index: index, quest: quest);
      if (!requestIsCurrent()) return;
      if (projection.projectId != projectId ||
          projection.projectRevision != projectRevision ||
          projection.checkpointIdentity != checkpointIdentity ||
          projection.questId != quest.id ||
          projection.questRevision != quest.revision) {
        throw const Revision3QuestJourneyStaleCheckpointException();
      }
      setState(() {
        _phase = _Revision3QuestJourneyViewPhase.available;
        _projection = projection;
      });
    } on Revision3QuestJourneyRequiresReopenException {
      if (!requestIsCurrent()) return;
      setState(() {
        _phase = _Revision3QuestJourneyViewPhase.requiresReopen;
        _projection = null;
      });
    } catch (_) {
      if (!requestIsCurrent()) return;
      setState(() {
        _phase = _Revision3QuestJourneyViewPhase.retryableUnavailable;
        _projection = null;
      });
    }
  }

  @override
  Widget build(BuildContext context) => switch (_phase) {
    _Revision3QuestJourneyViewPhase.loading => const Material(
      child: Center(
        child: SizedBox(
          key: Key('revision3-quest-journey-loading'),
          width: double.infinity,
          height: 240,
          child: Center(child: CircularProgressIndicator()),
        ),
      ),
    ),
    _Revision3QuestJourneyViewPhase.retryableUnavailable =>
      Revision3QuestJourneyPanel.unavailable(
        copy: widget.copy,
        onRetry: () => _beginLoad(notify: true),
        onEditNameObjectives: widget.onEditNameObjectives,
        onEditDescriptionConnections: widget.onEditDescriptionConnections,
        onEditStatesTransitions: widget.onEditStatesTransitions,
        editDisabledReason: widget.editDisabledReason,
        editNameObjectivesDisabledReason:
            widget.editNameObjectivesDisabledReason,
        editDescriptionConnectionsDisabledReason:
            widget.editDescriptionConnectionsDisabledReason,
        editStatesTransitionsDisabledReason:
            widget.editStatesTransitionsDisabledReason,
      ),
    _Revision3QuestJourneyViewPhase.requiresReopen =>
      Revision3QuestJourneyPanel.unavailable(
        copy: widget.copy,
        editDisabledReason:
            widget.editDisabledReason ??
            (widget.onEditNameObjectives != null ||
                    widget.onEditDescriptionConnections != null ||
                    widget.onEditStatesTransitions != null ||
                    widget.onOpenDialogVoice != null ||
                    widget.editNameObjectivesDisabledReason != null ||
                    widget.editDescriptionConnectionsDisabledReason != null ||
                    widget.editStatesTransitionsDisabledReason != null ||
                    widget.openDialogVoiceDisabledReason != null
                ? widget.copy.unavailableBody
                : null),
      ),
    _Revision3QuestJourneyViewPhase.available => Revision3QuestJourneyPanel(
      projection: _projection!,
      giverDisplayName: widget.giverDisplayName,
      parentStoryDisplayName: widget.parentStoryDisplayName,
      onEditNameObjectives: widget.onEditNameObjectives,
      onEditDescriptionConnections: widget.onEditDescriptionConnections,
      onEditStatesTransitions: widget.onEditStatesTransitions,
      onOpenDialogVoice: widget.onOpenDialogVoice,
      editDisabledReason: widget.editDisabledReason,
      editNameObjectivesDisabledReason: widget.editNameObjectivesDisabledReason,
      editDescriptionConnectionsDisabledReason:
          widget.editDescriptionConnectionsDisabledReason,
      editStatesTransitionsDisabledReason:
          widget.editStatesTransitionsDisabledReason,
      openDialogVoiceDisabledReason: widget.openDialogVoiceDisabledReason,
      onOpenDialogLine: widget.onOpenDialogLine,
      copy: widget.copy,
    ),
  };
}

void _requireExactInput({
  required String projectId,
  required int projectRevision,
  required String checkpointIdentity,
  required Revision3ContentIndex index,
  required Revision3ContentEntity quest,
}) {
  if (projectId.isEmpty ||
      projectRevision < 0 ||
      checkpointIdentity.isEmpty ||
      index.projectId != projectId ||
      index.projectRevision != projectRevision ||
      quest.kind != Revision3ContentEntityKind.questDraft ||
      quest.summary.questDraft == null ||
      quest.problemCount != 0 ||
      !identical(index.entityById(quest.id), quest)) {
    throw const Revision3QuestJourneyStaleCheckpointException();
  }
}
