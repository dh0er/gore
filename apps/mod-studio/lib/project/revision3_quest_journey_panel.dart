import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_quest_journey.dart';
import 'revision3_quest_transcript_authoring.dart';

typedef Revision3QuestJourneyAction = FutureOr<void> Function();
typedef Revision3QuestJourneyOpenDialogLine =
    FutureOr<void> Function(Revision3QuestTranscriptRow row);
typedef Revision3QuestJourneyCountCopy = String Function(int count);
typedef Revision3QuestJourneyObjectiveCopy = String Function(int ordinal);
typedef Revision3QuestJourneyTriggerCopy = String Function(int ruleCount);
typedef Revision3QuestJourneyFollowUpCopy =
    String Function(int count, bool completesParent);

/// All author-facing language rendered by [Revision3QuestJourneyPanel].
///
/// Exact project identities stay in callbacks and are never interpolated into
/// presentation copy. A localized owner can replace this copy without changing
/// the exact journey projection or gaining mutation authority here.
@immutable
final class Revision3QuestJourneyPanelCopy {
  const Revision3QuestJourneyPanelCopy({
    required this.questEyebrow,
    required this.untitledQuest,
    required this.draftLabel,
    required this.projectLogicLabel,
    required this.boundaryTitle,
    required this.boundaryBody,
    required this.giverLabel,
    required this.parentStoryLabel,
    required this.unknownGiver,
    required this.unknownParentStory,
    required this.editNameObjectives,
    required this.editDescriptionConnections,
    required this.editStatesTransitions,
    this.editActionBusyReason =
        'Another Quest action is still running. Wait for it to finish.',
    required this.mainQuestTitle,
    required this.mainQuestSubtitle,
    required this.objectivesTitle,
    required this.objectiveLabel,
    required this.originalBehaviorLabel,
    required this.originalBehaviorNote,
    required this.availableLabel,
    required this.startLabel,
    required this.successLabel,
    required this.failureLabel,
    required this.notUsed,
    required this.directTrigger,
    required this.automaticRules,
    required this.directOrAutomaticRules,
    required this.followUps,
    required this.objectiveDialogTitle,
    required this.objectiveDialogEmpty,
    required this.generalDialogTitle,
    required this.generalDialogEmpty,
    required this.legacyDialogBoundary,
    required this.showDialogLines,
    required this.hideDialogLines,
    required this.textLanguageCount,
    required this.voiceTakeCount,
    required this.selectedVoiceCount,
    required this.sharedQuestCount,
    required this.dialogLineLabel,
    required this.openDialogLineTooltip,
    required this.actionFailed,
    required this.unavailableTitle,
    required this.unavailableBody,
    required this.retryLabel,
  });

  const Revision3QuestJourneyPanelCopy.english()
    : questEyebrow = 'Quest journey',
      untitledQuest = 'Untitled Quest',
      draftLabel = 'Draft',
      projectLogicLabel = 'Project logic',
      boundaryTitle = 'Offline project view',
      boundaryBody =
          'This shows authored project logic only. It does not prove that the Quest builds, deploys, runs in the game, or works with a save.',
      giverLabel = 'Quest giver',
      parentStoryLabel = 'Part of',
      unknownGiver = 'Linked character',
      unknownParentStory = 'Linked story',
      editNameObjectives = 'Edit name & objectives',
      editDescriptionConnections = 'Edit description & connections',
      editStatesTransitions = 'Edit states & transitions',
      editActionBusyReason =
          'Another Quest action is still running. Wait for it to finish.',
      mainQuestTitle = 'Main Quest',
      mainQuestSubtitle =
          'How the Quest itself can become available, start, succeed, or fail.',
      objectivesTitle = 'Objectives',
      objectiveLabel = _englishObjectiveLabel,
      originalBehaviorLabel = 'Original fixed behavior',
      originalBehaviorNote =
          'This older Quest has an effective project behavior, but it does not store stable objective links for dialog.',
      availableLabel = 'Available',
      startLabel = 'Start',
      successLabel = 'Success',
      failureLabel = 'Failure',
      notUsed = 'Not used',
      directTrigger = 'Direct trigger allowed',
      automaticRules = _englishAutomaticRules,
      directOrAutomaticRules = _englishDirectOrAutomaticRules,
      followUps = _englishFollowUps,
      objectiveDialogTitle = 'Linked dialog',
      objectiveDialogEmpty = 'No dialog is linked to this objective.',
      generalDialogTitle = 'General dialog',
      generalDialogEmpty = 'No general dialog is linked to this Quest.',
      legacyDialogBoundary =
          'This older Quest has no stored objective-to-dialog links. Its dialog stays here instead of being guessed into objectives.',
      showDialogLines = _englishShowDialogLines,
      hideDialogLines = 'Hide dialog lines',
      textLanguageCount = _englishTextLanguageCount,
      voiceTakeCount = _englishVoiceTakeCount,
      selectedVoiceCount = _englishSelectedVoiceCount,
      sharedQuestCount = _englishSharedQuestCount,
      dialogLineLabel = _englishDialogLineLabel,
      openDialogLineTooltip = 'Open dialog text & Voice',
      actionFailed =
          'That editor could not be opened. The project view was not changed.',
      unavailableTitle = 'Quest journey unavailable',
      unavailableBody =
          'The exact project checkpoint could not be verified. Refresh the project or reopen it before editing this Quest.',
      retryLabel = 'Retry';

  const Revision3QuestJourneyPanelCopy.german()
    : questEyebrow = 'Quest-Ablauf',
      untitledQuest = 'Unbenannte Quest',
      draftLabel = 'Entwurf',
      projectLogicLabel = 'Projektlogik',
      boundaryTitle = 'Offline-Projektansicht',
      boundaryBody =
          'Diese Ansicht zeigt nur die entworfene Projektlogik. Sie belegt nicht, dass die Quest gebaut, ins Spiel übertragen oder dort ausgeführt werden kann oder mit einem Spielstand funktioniert.',
      giverLabel = 'Questgeber',
      parentStoryLabel = 'Teil von',
      unknownGiver = 'Verknüpfte Figur',
      unknownParentStory = 'Verknüpfte Handlung',
      editNameObjectives = 'Name & Ziele bearbeiten',
      editDescriptionConnections = 'Beschreibung & Verknüpfungen bearbeiten',
      editStatesTransitions = 'Zustände & Übergänge bearbeiten',
      editActionBusyReason =
          'Eine andere Quest-Aktion läuft noch. Warte, bis sie abgeschlossen ist.',
      mainQuestTitle = 'Hauptquest',
      mainQuestSubtitle =
          'Wie die Quest selbst verfügbar werden, starten, erfolgreich enden oder fehlschlagen kann.',
      objectivesTitle = 'Ziele',
      objectiveLabel = _germanObjectiveLabel,
      originalBehaviorLabel = 'Ursprüngliches festes Verhalten',
      originalBehaviorNote =
          'Diese ältere Quest besitzt ein wirksames Projektverhalten, speichert aber keine stabilen Zielverknüpfungen für Dialogzeilen.',
      availableLabel = 'Verfügbar',
      startLabel = 'Start',
      successLabel = 'Erfolg',
      failureLabel = 'Fehlschlag',
      notUsed = 'Nicht verwendet',
      directTrigger = 'Kann direkt ausgelöst werden',
      automaticRules = _germanAutomaticRules,
      directOrAutomaticRules = _germanDirectOrAutomaticRules,
      followUps = _germanFollowUps,
      objectiveDialogTitle = 'Verknüpfte Dialogzeilen',
      objectiveDialogEmpty =
          'Mit diesem Ziel sind keine Dialogzeilen verknüpft.',
      generalDialogTitle = 'Allgemeiner Dialog',
      generalDialogEmpty =
          'Mit dieser Quest sind keine allgemeinen Dialogzeilen verknüpft.',
      legacyDialogBoundary =
          'Diese ältere Quest speichert keine Verknüpfungen zwischen Zielen und Dialogzeilen. Ihre Dialogzeilen bleiben deshalb hier, statt Zielen nur vermutungsweise zugeordnet zu werden.',
      showDialogLines = _germanShowDialogLines,
      hideDialogLines = 'Dialogzeilen ausblenden',
      textLanguageCount = _germanTextLanguageCount,
      voiceTakeCount = _germanVoiceTakeCount,
      selectedVoiceCount = _germanSelectedVoiceCount,
      sharedQuestCount = _germanSharedQuestCount,
      dialogLineLabel = _germanDialogLineLabel,
      openDialogLineTooltip = 'Dialogtext & Sprachausgabe öffnen',
      actionFailed =
          'Der gewählte Editor konnte nicht geöffnet werden. Die Projektansicht wurde nicht verändert.',
      unavailableTitle = 'Quest-Ablauf nicht verfügbar',
      unavailableBody =
          'Der genaue Projektstand konnte nicht bestätigt werden. Aktualisiere oder öffne das Projekt erneut, bevor du diese Quest bearbeitest.',
      retryLabel = 'Erneut versuchen';

  final String questEyebrow;
  final String untitledQuest;
  final String draftLabel;
  final String projectLogicLabel;
  final String boundaryTitle;
  final String boundaryBody;
  final String giverLabel;
  final String parentStoryLabel;
  final String unknownGiver;
  final String unknownParentStory;
  final String editNameObjectives;
  final String editDescriptionConnections;
  final String editStatesTransitions;
  final String editActionBusyReason;
  final String mainQuestTitle;
  final String mainQuestSubtitle;
  final String objectivesTitle;
  final Revision3QuestJourneyObjectiveCopy objectiveLabel;
  final String originalBehaviorLabel;
  final String originalBehaviorNote;
  final String availableLabel;
  final String startLabel;
  final String successLabel;
  final String failureLabel;
  final String notUsed;
  final String directTrigger;
  final Revision3QuestJourneyTriggerCopy automaticRules;
  final Revision3QuestJourneyTriggerCopy directOrAutomaticRules;
  final Revision3QuestJourneyFollowUpCopy followUps;
  final String objectiveDialogTitle;
  final String objectiveDialogEmpty;
  final String generalDialogTitle;
  final String generalDialogEmpty;
  final String legacyDialogBoundary;
  final Revision3QuestJourneyCountCopy showDialogLines;
  final String hideDialogLines;
  final Revision3QuestJourneyCountCopy textLanguageCount;
  final Revision3QuestJourneyCountCopy voiceTakeCount;
  final Revision3QuestJourneyCountCopy selectedVoiceCount;
  final Revision3QuestJourneyCountCopy sharedQuestCount;
  final Revision3QuestJourneyObjectiveCopy dialogLineLabel;
  final String openDialogLineTooltip;
  final String actionFailed;
  final String unavailableTitle;
  final String unavailableBody;
  final String retryLabel;
}

/// Responsive, objective-centered presentation of one exact Quest journey.
///
/// This widget owns no project writer and no runtime authority. Editing and
/// exact DialogLine navigation are explicit owner-supplied callbacks.
class Revision3QuestJourneyPanel extends StatefulWidget {
  const Revision3QuestJourneyPanel({
    required Revision3QuestJourneyProjection this.projection,
    this.giverDisplayName,
    this.parentStoryDisplayName,
    this.onEditNameObjectives,
    this.onEditDescriptionConnections,
    this.onEditStatesTransitions,
    this.editDisabledReason,
    this.editNameObjectivesDisabledReason,
    this.editDescriptionConnectionsDisabledReason,
    this.editStatesTransitionsDisabledReason,
    this.onOpenDialogLine,
    this.copy = const Revision3QuestJourneyPanelCopy.english(),
    super.key,
  }) : onRetry = null;

  const Revision3QuestJourneyPanel.unavailable({
    this.onRetry,
    this.onEditNameObjectives,
    this.onEditDescriptionConnections,
    this.onEditStatesTransitions,
    this.editDisabledReason,
    this.editNameObjectivesDisabledReason,
    this.editDescriptionConnectionsDisabledReason,
    this.editStatesTransitionsDisabledReason,
    this.copy = const Revision3QuestJourneyPanelCopy.english(),
    super.key,
  }) : projection = null,
       giverDisplayName = null,
       parentStoryDisplayName = null,
       onOpenDialogLine = null;

  final Revision3QuestJourneyProjection? projection;

  /// Optional display-safe labels resolved by the owner. When omitted, the
  /// panel derives a conservative friendly label from the runtime name.
  final String? giverDisplayName;
  final String? parentStoryDisplayName;
  final Revision3QuestJourneyAction? onEditNameObjectives;
  final Revision3QuestJourneyAction? onEditDescriptionConnections;
  final Revision3QuestJourneyAction? onEditStatesTransitions;

  /// Localized owner-provided reason that disables all three edit actions.
  ///
  /// Supplying a non-empty reason preserves the edit controls even when their
  /// callbacks are unavailable (for example while the project is dirty, busy,
  /// or requires reopening). Supplying neither callbacks nor a reason keeps a
  /// deliberately read-only journey free of edit controls.
  final String? editDisabledReason;

  /// Optional localized reasons that disable only their matching action.
  ///
  /// A non-empty per-action reason keeps that action visible even when its
  /// callback is null. [editDisabledReason] remains the global override.
  final String? editNameObjectivesDisabledReason;
  final String? editDescriptionConnectionsDisabledReason;
  final String? editStatesTransitionsDisabledReason;
  final Revision3QuestJourneyOpenDialogLine? onOpenDialogLine;
  final Revision3QuestJourneyAction? onRetry;
  final Revision3QuestJourneyPanelCopy copy;

  @override
  State<Revision3QuestJourneyPanel> createState() =>
      _Revision3QuestJourneyPanelState();
}

final class _Revision3QuestJourneyPanelState
    extends State<Revision3QuestJourneyPanel> {
  String? _busyAction;
  String? _actionError;

  @override
  void didUpdateWidget(covariant Revision3QuestJourneyPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldProjection = oldWidget.projection;
    final projection = widget.projection;
    if (oldProjection?.checkpointIdentity != projection?.checkpointIdentity ||
        oldProjection?.projectRevision != projection?.projectRevision ||
        oldProjection?.questRevision != projection?.questRevision ||
        oldProjection?.moduleRevision != projection?.moduleRevision) {
      _busyAction = null;
      _actionError = null;
    }
  }

  Future<void> _runAction(
    String actionKey,
    Revision3QuestJourneyAction action,
  ) async {
    if (_busyAction != null) return;
    setState(() {
      _busyAction = actionKey;
      _actionError = null;
    });
    try {
      await Future<void>.sync(action);
      if (!mounted) return;
      setState(() => _busyAction = null);
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busyAction = null;
        _actionError = widget.copy.actionFailed;
      });
    }
  }

  Future<void> _openDialogLine(Revision3QuestJourneyDialogLine line) async {
    final action = widget.onOpenDialogLine;
    if (action == null) return;
    await _runAction('dialog-${line.transcriptIndex}', () => action(line.row));
  }

  @override
  Widget build(BuildContext context) {
    final projection = widget.projection;
    if (projection == null) {
      return _UnavailableJourney(
        copy: widget.copy,
        busyAction: _busyAction,
        actionError: _actionError,
        editDisabledReason: widget.editDisabledReason,
        editNameObjectivesDisabledReason:
            widget.editNameObjectivesDisabledReason,
        editDescriptionConnectionsDisabledReason:
            widget.editDescriptionConnectionsDisabledReason,
        editStatesTransitionsDisabledReason:
            widget.editStatesTransitionsDisabledReason,
        onRetry: widget.onRetry == null
            ? null
            : () => _runAction('retry', widget.onRetry!),
        onEditNameObjectives: widget.onEditNameObjectives == null
            ? null
            : () => _runAction('name-objectives', widget.onEditNameObjectives!),
        onEditDescriptionConnections:
            widget.onEditDescriptionConnections == null
            ? null
            : () => _runAction(
                'description-connections',
                widget.onEditDescriptionConnections!,
              ),
        onEditStatesTransitions: widget.onEditStatesTransitions == null
            ? null
            : () => _runAction(
                'states-transitions',
                widget.onEditStatesTransitions!,
              ),
      );
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 900;
        return Material(
          key: const Key('revision3-quest-journey-panel'),
          color: Theme.of(context).colorScheme.surface,
          child: SingleChildScrollView(
            child: Padding(
              padding: EdgeInsets.all(wide ? 24 : 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  KeyedSubtree(
                    key: Key(
                      wide
                          ? 'revision3-quest-journey-wide'
                          : 'revision3-quest-journey-narrow',
                    ),
                    child: _JourneyHeader(
                      projection: projection,
                      giverDisplayName: widget.giverDisplayName,
                      parentStoryDisplayName: widget.parentStoryDisplayName,
                      copy: widget.copy,
                      busyAction: _busyAction,
                      editDisabledReason: widget.editDisabledReason,
                      editNameObjectivesDisabledReason:
                          widget.editNameObjectivesDisabledReason,
                      editDescriptionConnectionsDisabledReason:
                          widget.editDescriptionConnectionsDisabledReason,
                      editStatesTransitionsDisabledReason:
                          widget.editStatesTransitionsDisabledReason,
                      onEditNameObjectives: widget.onEditNameObjectives == null
                          ? null
                          : () => _runAction(
                              'name-objectives',
                              widget.onEditNameObjectives!,
                            ),
                      onEditDescriptionConnections:
                          widget.onEditDescriptionConnections == null
                          ? null
                          : () => _runAction(
                              'description-connections',
                              widget.onEditDescriptionConnections!,
                            ),
                      onEditStatesTransitions:
                          widget.onEditStatesTransitions == null
                          ? null
                          : () => _runAction(
                              'states-transitions',
                              widget.onEditStatesTransitions!,
                            ),
                    ),
                  ),
                  if (_busyAction != null) ...<Widget>[
                    const SizedBox(height: 12),
                    _JourneyBusyStatus(copy: widget.copy),
                  ],
                  const SizedBox(height: 16),
                  _AuthorityBoundary(copy: widget.copy),
                  if (_actionError != null) ...<Widget>[
                    const SizedBox(height: 12),
                    Semantics(
                      liveRegion: true,
                      child: Text(
                        _actionError!,
                        key: const Key('revision3-quest-journey-action-error'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ),
                  ],
                  const SizedBox(height: 20),
                  _JourneyBehaviorCard(
                    key: const Key('revision3-quest-journey-main-behavior'),
                    title: widget.copy.mainQuestTitle,
                    subtitle: widget.copy.mainQuestSubtitle,
                    behavior: projection.rootBehavior,
                    behaviorKey: 'main',
                    copy: widget.copy,
                  ),
                  const SizedBox(height: 20),
                  if (wide)
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Expanded(
                          child: _ObjectivesColumn(
                            projection: projection,
                            copy: widget.copy,
                            busyAction: _busyAction,
                            canOpenDialog: widget.onOpenDialogLine != null,
                            onOpenDialog: _openDialogLine,
                          ),
                        ),
                        const SizedBox(width: 20),
                        SizedBox(
                          width: 340,
                          child: _GeneralDialogCard(
                            projection: projection,
                            copy: widget.copy,
                            busyAction: _busyAction,
                            canOpenDialog: widget.onOpenDialogLine != null,
                            onOpenDialog: _openDialogLine,
                          ),
                        ),
                      ],
                    )
                  else ...<Widget>[
                    _ObjectivesColumn(
                      projection: projection,
                      copy: widget.copy,
                      busyAction: _busyAction,
                      canOpenDialog: widget.onOpenDialogLine != null,
                      onOpenDialog: _openDialogLine,
                    ),
                    const SizedBox(height: 16),
                    _GeneralDialogCard(
                      projection: projection,
                      copy: widget.copy,
                      busyAction: _busyAction,
                      canOpenDialog: widget.onOpenDialogLine != null,
                      onOpenDialog: _openDialogLine,
                    ),
                  ],
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

final class _JourneyHeader extends StatelessWidget {
  const _JourneyHeader({
    required this.projection,
    required this.giverDisplayName,
    required this.parentStoryDisplayName,
    required this.copy,
    required this.busyAction,
    required this.editDisabledReason,
    required this.editNameObjectivesDisabledReason,
    required this.editDescriptionConnectionsDisabledReason,
    required this.editStatesTransitionsDisabledReason,
    required this.onEditNameObjectives,
    required this.onEditDescriptionConnections,
    required this.onEditStatesTransitions,
  });

  final Revision3QuestJourneyProjection projection;
  final String? giverDisplayName;
  final String? parentStoryDisplayName;
  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final String? editDisabledReason;
  final String? editNameObjectivesDisabledReason;
  final String? editDescriptionConnectionsDisabledReason;
  final String? editStatesTransitionsDisabledReason;
  final VoidCallback? onEditNameObjectives;
  final VoidCallback? onEditDescriptionConnections;
  final VoidCallback? onEditStatesTransitions;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final title = projection.title.trim().isEmpty
        ? copy.untitledQuest
        : projection.title;
    final giver = giverDisplayName?.trim().isNotEmpty == true
        ? giverDisplayName!.trim()
        : _friendlyRuntimeLabel(
            projection.giverRuntimeUniqueName,
            fallback: copy.unknownGiver,
          );
    final parent = parentStoryDisplayName?.trim().isNotEmpty == true
        ? parentStoryDisplayName!.trim()
        : _friendlyRuntimeLabel(
            projection.parentRuntimeClass,
            fallback: copy.unknownParentStory,
          );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          copy.questEyebrow.toUpperCase(),
          style: theme.textTheme.labelMedium?.copyWith(
            color: theme.colorScheme.primary,
            letterSpacing: 1.1,
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          title,
          key: const Key('revision3-quest-journey-title'),
          style: theme.textTheme.headlineMedium?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 12),
        Wrap(
          spacing: 10,
          runSpacing: 8,
          children: <Widget>[
            _ContextChip(
              icon: Icons.person_outline,
              label: copy.giverLabel,
              value: giver,
              key: const Key('revision3-quest-journey-giver'),
            ),
            _ContextChip(
              icon: Icons.account_tree_outlined,
              label: copy.parentStoryLabel,
              value: parent,
              key: const Key('revision3-quest-journey-parent'),
            ),
          ],
        ),
        if (onEditNameObjectives != null ||
            onEditDescriptionConnections != null ||
            onEditStatesTransitions != null ||
            _nonEmptyReason(editDisabledReason) != null ||
            _nonEmptyReason(editNameObjectivesDisabledReason) != null ||
            _nonEmptyReason(editDescriptionConnectionsDisabledReason) != null ||
            _nonEmptyReason(editStatesTransitionsDisabledReason) !=
                null) ...<Widget>[
          const SizedBox(height: 16),
          _JourneyEditActions(
            copy: copy,
            busyAction: busyAction,
            editDisabledReason: editDisabledReason,
            editNameObjectivesDisabledReason: editNameObjectivesDisabledReason,
            editDescriptionConnectionsDisabledReason:
                editDescriptionConnectionsDisabledReason,
            editStatesTransitionsDisabledReason:
                editStatesTransitionsDisabledReason,
            onEditNameObjectives: onEditNameObjectives,
            onEditDescriptionConnections: onEditDescriptionConnections,
            onEditStatesTransitions: onEditStatesTransitions,
          ),
        ],
      ],
    );
  }
}

final class _JourneyEditActions extends StatelessWidget {
  const _JourneyEditActions({
    required this.copy,
    required this.busyAction,
    required this.editDisabledReason,
    required this.editNameObjectivesDisabledReason,
    required this.editDescriptionConnectionsDisabledReason,
    required this.editStatesTransitionsDisabledReason,
    required this.onEditNameObjectives,
    required this.onEditDescriptionConnections,
    required this.onEditStatesTransitions,
    this.center = false,
    this.visibleReasonExclusions = const <String>{},
  });

  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final String? editDisabledReason;
  final String? editNameObjectivesDisabledReason;
  final String? editDescriptionConnectionsDisabledReason;
  final String? editStatesTransitionsDisabledReason;
  final VoidCallback? onEditNameObjectives;
  final VoidCallback? onEditDescriptionConnections;
  final VoidCallback? onEditStatesTransitions;
  final bool center;
  final Set<String> visibleReasonExclusions;

  @override
  Widget build(BuildContext context) {
    final globalReason = _nonEmptyReason(editDisabledReason);
    final busyReason = busyAction == null ? null : copy.editActionBusyReason;
    final nameReason = _nonEmptyReason(editNameObjectivesDisabledReason);
    final connectionsReason = _nonEmptyReason(
      editDescriptionConnectionsDisabledReason,
    );
    final transitionsReason = _nonEmptyReason(
      editStatesTransitionsDisabledReason,
    );
    final showAll = globalReason != null;
    final candidateVisibleReasons = globalReason == null
        ? <String>{
            ?nameReason,
            ?connectionsReason,
            ?transitionsReason,
          }.toList(growable: false)
        : <String>[globalReason];
    final visibleReasons = candidateVisibleReasons
        .where((reason) => !visibleReasonExclusions.contains(reason))
        .toList(growable: false);

    String? effectiveReason(String? actionReason) =>
        globalReason ?? busyReason ?? actionReason;

    Widget explainDisabled(Widget button, String? reason) =>
        reason == null ? button : Tooltip(message: reason, child: button);

    VoidCallback? enabledCallback(
      VoidCallback? callback,
      String? actionReason,
    ) => callback != null && effectiveReason(actionReason) == null
        ? callback
        : null;

    return Column(
      crossAxisAlignment: center
          ? CrossAxisAlignment.center
          : CrossAxisAlignment.start,
      children: <Widget>[
        Wrap(
          alignment: center ? WrapAlignment.center : WrapAlignment.start,
          spacing: 8,
          runSpacing: 8,
          children: <Widget>[
            if (showAll || onEditNameObjectives != null || nameReason != null)
              explainDisabled(
                OutlinedButton.icon(
                  key: const Key(
                    'revision3-quest-journey-edit-name-objectives',
                  ),
                  onPressed: enabledCallback(onEditNameObjectives, nameReason),
                  icon: _ActionIcon(
                    busy: busyAction == 'name-objectives',
                    fallback: Icons.edit_outlined,
                  ),
                  label: Text(copy.editNameObjectives),
                ),
                effectiveReason(nameReason),
              ),
            if (showAll ||
                onEditDescriptionConnections != null ||
                connectionsReason != null)
              explainDisabled(
                OutlinedButton.icon(
                  key: const Key(
                    'revision3-quest-journey-edit-description-connections',
                  ),
                  onPressed: enabledCallback(
                    onEditDescriptionConnections,
                    connectionsReason,
                  ),
                  icon: _ActionIcon(
                    busy: busyAction == 'description-connections',
                    fallback: Icons.hub_outlined,
                  ),
                  label: Text(copy.editDescriptionConnections),
                ),
                effectiveReason(connectionsReason),
              ),
            if (showAll ||
                onEditStatesTransitions != null ||
                transitionsReason != null)
              explainDisabled(
                FilledButton.tonalIcon(
                  key: const Key(
                    'revision3-quest-journey-edit-states-transitions',
                  ),
                  onPressed: enabledCallback(
                    onEditStatesTransitions,
                    transitionsReason,
                  ),
                  icon: _ActionIcon(
                    busy: busyAction == 'states-transitions',
                    fallback: Icons.schema_outlined,
                  ),
                  label: Text(copy.editStatesTransitions),
                ),
                effectiveReason(transitionsReason),
              ),
          ],
        ),
        if (visibleReasons.isNotEmpty) ...<Widget>[
          const SizedBox(height: 8),
          Container(
            key: const Key('revision3-quest-journey-edit-disabled-reason'),
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Column(
              crossAxisAlignment: center
                  ? CrossAxisAlignment.center
                  : CrossAxisAlignment.start,
              children: <Widget>[
                for (
                  var index = 0;
                  index < visibleReasons.length;
                  index++
                ) ...<Widget>[
                  if (index > 0) const SizedBox(height: 6),
                  Text(
                    visibleReasons[index],
                    textAlign: center ? TextAlign.center : TextAlign.start,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }
}

String? _nonEmptyReason(String? reason) {
  final normalized = reason?.trim();
  return normalized?.isNotEmpty == true ? normalized : null;
}

final class _JourneyBusyStatus extends StatelessWidget {
  const _JourneyBusyStatus({required this.copy});

  final Revision3QuestJourneyPanelCopy copy;

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-quest-journey-action-progress'),
    container: true,
    liveRegion: true,
    child: Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(10),
      ),
      child: Row(
        children: <Widget>[
          SizedBox.square(
            dimension: 18,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              semanticsLabel: copy.editActionBusyReason,
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: ExcludeSemantics(
              child: Text(
                copy.editActionBusyReason,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
        ],
      ),
    ),
  );
}

final class _ActionIcon extends StatelessWidget {
  const _ActionIcon({required this.busy, required this.fallback});

  final bool busy;
  final IconData fallback;

  @override
  Widget build(BuildContext context) => busy
      ? const SizedBox.square(
          dimension: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        )
      : Icon(fallback);
}

final class _ContextChip extends StatelessWidget {
  const _ContextChip({
    required this.icon,
    required this.label,
    required this.value,
    super.key,
  });

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(999),
      border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 18),
        const SizedBox(width: 7),
        Text('$label: ', style: const TextStyle(fontWeight: FontWeight.w600)),
        Flexible(child: Text(value, overflow: TextOverflow.ellipsis)),
      ],
    ),
  );
}

final class _AuthorityBoundary extends StatelessWidget {
  const _AuthorityBoundary({required this.copy});

  final Revision3QuestJourneyPanelCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key('revision3-quest-journey-boundary'),
      color: scheme.tertiaryContainer,
      borderRadius: BorderRadius.circular(14),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(Icons.science_outlined, color: scheme.onTertiaryContainer),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: <Widget>[
                      _BoundaryBadge(label: copy.draftLabel),
                      _BoundaryBadge(label: copy.projectLogicLabel),
                    ],
                  ),
                  const SizedBox(height: 7),
                  Text(
                    copy.boundaryTitle,
                    style: const TextStyle(fontWeight: FontWeight.w700),
                  ),
                  const SizedBox(height: 2),
                  Text(copy.boundaryBody),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

final class _BoundaryBadge extends StatelessWidget {
  const _BoundaryBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surface.withValues(alpha: 0.72),
      borderRadius: BorderRadius.circular(999),
    ),
    child: Text(
      label,
      style: Theme.of(
        context,
      ).textTheme.labelSmall?.copyWith(fontWeight: FontWeight.w700),
    ),
  );
}

final class _ObjectivesColumn extends StatelessWidget {
  const _ObjectivesColumn({
    required this.projection,
    required this.copy,
    required this.busyAction,
    required this.canOpenDialog,
    required this.onOpenDialog,
  });

  final Revision3QuestJourneyProjection projection;
  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final bool canOpenDialog;
  final ValueChanged<Revision3QuestJourneyDialogLine> onOpenDialog;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: <Widget>[
      Text(copy.objectivesTitle, style: Theme.of(context).textTheme.titleLarge),
      const SizedBox(height: 10),
      for (
        var index = 0;
        index < projection.objectives.length;
        index++
      ) ...<Widget>[
        _ObjectiveCard(
          key: Key('revision3-quest-journey-objective-$index'),
          ordinal: index + 1,
          objective: projection.objectives[index],
          legacy: projection.legacySyntheticBehavior,
          copy: copy,
          busyAction: busyAction,
          canOpenDialog: canOpenDialog,
          onOpenDialog: onOpenDialog,
        ),
        if (index + 1 < projection.objectives.length)
          const SizedBox(height: 12),
      ],
    ],
  );
}

final class _ObjectiveCard extends StatelessWidget {
  const _ObjectiveCard({
    required this.ordinal,
    required this.objective,
    required this.legacy,
    required this.copy,
    required this.busyAction,
    required this.canOpenDialog,
    required this.onOpenDialog,
    super.key,
  });

  final int ordinal;
  final Revision3QuestJourneyObjective objective;
  final bool legacy;
  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final bool canOpenDialog;
  final ValueChanged<Revision3QuestJourneyDialogLine> onOpenDialog;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(14),
        side: BorderSide(color: scheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            LayoutBuilder(
              builder: (context, constraints) {
                final title = _ObjectiveTitle(
                  ordinal: ordinal,
                  title: objective.title,
                  label: copy.objectiveLabel(ordinal),
                  scheme: scheme,
                );
                if (!legacy) return title;
                final badge = _SmallBadge(
                  icon: Icons.history,
                  label: copy.originalBehaviorLabel,
                );
                if (constraints.maxWidth < 520) {
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[title, const SizedBox(height: 8), badge],
                  );
                }
                return Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Expanded(child: title),
                    const SizedBox(width: 10),
                    badge,
                  ],
                );
              },
            ),
            if (legacy) ...<Widget>[
              const SizedBox(height: 10),
              Text(copy.originalBehaviorNote),
            ],
            const SizedBox(height: 14),
            _BehaviorOverview(
              behavior: objective.behavior,
              behaviorKey: 'objective-${ordinal - 1}',
              copy: copy,
            ),
            const SizedBox(height: 14),
            _DialogGroup(
              key: Key(
                'revision3-quest-journey-objective-dialog-${ordinal - 1}',
              ),
              title: copy.objectiveDialogTitle,
              emptyMessage: legacy
                  ? copy.legacyDialogBoundary
                  : copy.objectiveDialogEmpty,
              lines: objective.dialogLines,
              copy: copy,
              busyAction: busyAction,
              canOpenDialog: canOpenDialog,
              onOpenDialog: onOpenDialog,
            ),
          ],
        ),
      ),
    );
  }
}

final class _ObjectiveTitle extends StatelessWidget {
  const _ObjectiveTitle({
    required this.ordinal,
    required this.title,
    required this.label,
    required this.scheme,
  });

  final int ordinal;
  final String title;
  final String label;
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      CircleAvatar(
        radius: 17,
        backgroundColor: scheme.primaryContainer,
        foregroundColor: scheme.onPrimaryContainer,
        child: Text('$ordinal'),
      ),
      const SizedBox(width: 11),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              label,
              style: Theme.of(
                context,
              ).textTheme.labelMedium?.copyWith(color: scheme.onSurfaceVariant),
            ),
            Text(
              title,
              key: Key(
                'revision3-quest-journey-objective-title-${ordinal - 1}',
              ),
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
    ],
  );
}

final class _JourneyBehaviorCard extends StatelessWidget {
  const _JourneyBehaviorCard({
    required this.title,
    required this.subtitle,
    required this.behavior,
    required this.behaviorKey,
    required this.copy,
    super.key,
  });

  final String title;
  final String subtitle;
  final Revision3QuestJourneyNodeBehavior behavior;
  final String behaviorKey;
  final Revision3QuestJourneyPanelCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(14),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 2),
            Text(subtitle),
            const SizedBox(height: 14),
            _BehaviorOverview(
              behavior: behavior,
              behaviorKey: behaviorKey,
              copy: copy,
            ),
          ],
        ),
      ),
    );
  }
}

final class _BehaviorOverview extends StatelessWidget {
  const _BehaviorOverview({
    required this.behavior,
    required this.behaviorKey,
    required this.copy,
  });

  final Revision3QuestJourneyNodeBehavior behavior;
  final String behaviorKey;
  final Revision3QuestJourneyPanelCopy copy;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final columns = constraints.maxWidth >= 620 ? 4 : 2;
      const gap = 8.0;
      final width = (constraints.maxWidth - gap * (columns - 1)) / columns;
      final entries = <(String, AuthoringRevision3QuestTransitionV1?, String)>[
        (copy.availableLabel, behavior.availability, 'availability'),
        (copy.startLabel, behavior.start, 'start'),
        (copy.successLabel, behavior.success, 'success'),
        (copy.failureLabel, behavior.failure, 'failure'),
      ];
      return Wrap(
        spacing: gap,
        runSpacing: gap,
        children: <Widget>[
          for (final entry in entries)
            SizedBox(
              width: width,
              child: _BehaviorCell(
                key: Key(
                  'revision3-quest-journey-behavior-$behaviorKey-${entry.$3}',
                ),
                label: entry.$1,
                transition: entry.$2,
                copy: copy,
              ),
            ),
        ],
      );
    },
  );
}

final class _BehaviorCell extends StatelessWidget {
  const _BehaviorCell({
    required this.label,
    required this.transition,
    required this.copy,
    super.key,
  });

  final String label;
  final AuthoringRevision3QuestTransitionV1? transition;
  final Revision3QuestJourneyPanelCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final active = transition != null;
    return Container(
      constraints: const BoxConstraints(minHeight: 82),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: active
            ? scheme.secondaryContainer.withValues(alpha: 0.55)
            : scheme.surfaceContainerHighest.withValues(alpha: 0.6),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: Theme.of(
              context,
            ).textTheme.labelMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 4),
          Text(
            _transitionSummary(transition, copy),
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}

final class _GeneralDialogCard extends StatelessWidget {
  const _GeneralDialogCard({
    required this.projection,
    required this.copy,
    required this.busyAction,
    required this.canOpenDialog,
    required this.onOpenDialog,
  });

  final Revision3QuestJourneyProjection projection;
  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final bool canOpenDialog;
  final ValueChanged<Revision3QuestJourneyDialogLine> onOpenDialog;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key('revision3-quest-journey-general-dialog'),
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(14),
        side: BorderSide(color: scheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            if (projection.legacySyntheticBehavior) ...<Widget>[
              _InlineNotice(
                key: const Key('revision3-quest-journey-legacy-dialog-note'),
                text: copy.legacyDialogBoundary,
              ),
              const SizedBox(height: 10),
            ],
            _DialogGroup(
              title: copy.generalDialogTitle,
              emptyMessage: copy.generalDialogEmpty,
              lines: projection.generalDialogLines,
              copy: copy,
              busyAction: busyAction,
              canOpenDialog: canOpenDialog,
              onOpenDialog: onOpenDialog,
            ),
          ],
        ),
      ),
    );
  }
}

final class _DialogGroup extends StatefulWidget {
  const _DialogGroup({
    required this.title,
    required this.emptyMessage,
    required this.lines,
    required this.copy,
    required this.busyAction,
    required this.canOpenDialog,
    required this.onOpenDialog,
    super.key,
  });

  final String title;
  final String emptyMessage;
  final List<Revision3QuestJourneyDialogLine> lines;
  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final bool canOpenDialog;
  final ValueChanged<Revision3QuestJourneyDialogLine> onOpenDialog;

  @override
  State<_DialogGroup> createState() => _DialogGroupState();
}

final class _DialogGroupState extends State<_DialogGroup> {
  late bool _expanded = widget.lines.length <= 4;

  @override
  void didUpdateWidget(covariant _DialogGroup oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.lines.length != widget.lines.length) {
      _expanded = widget.lines.length <= 4;
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: <Widget>[
      Row(
        children: <Widget>[
          Expanded(
            child: Text(
              widget.title,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
            ),
          ),
          if (widget.lines.isNotEmpty)
            _SmallBadge(
              icon: Icons.forum_outlined,
              label: '${widget.lines.length}',
            ),
        ],
      ),
      const SizedBox(height: 7),
      if (widget.lines.isEmpty)
        Text(
          widget.emptyMessage,
          key: const Key('revision3-quest-journey-dialog-empty'),
          style: TextStyle(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        )
      else if (!_expanded)
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            onPressed: () => setState(() => _expanded = true),
            icon: const Icon(Icons.expand_more),
            label: Text(widget.copy.showDialogLines(widget.lines.length)),
          ),
        )
      else ...<Widget>[
        for (var index = 0; index < widget.lines.length; index++) ...<Widget>[
          _DialogLineTile(
            line: widget.lines[index],
            copy: widget.copy,
            busy:
                widget.busyAction ==
                'dialog-${widget.lines[index].transcriptIndex}',
            interactive: widget.canOpenDialog,
            enabled: widget.canOpenDialog && widget.busyAction == null,
            onTap: () => widget.onOpenDialog(widget.lines[index]),
          ),
          if (index + 1 < widget.lines.length) const SizedBox(height: 6),
        ],
        if (widget.lines.length > 4)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              onPressed: () => setState(() => _expanded = false),
              icon: const Icon(Icons.expand_less),
              label: Text(widget.copy.hideDialogLines),
            ),
          ),
      ],
    ],
  );
}

final class _DialogLineTile extends StatelessWidget {
  const _DialogLineTile({
    required this.line,
    required this.copy,
    required this.busy,
    required this.interactive,
    required this.enabled,
    required this.onTap,
  });

  final Revision3QuestJourneyDialogLine line;
  final Revision3QuestJourneyPanelCopy copy;
  final bool busy;
  final bool interactive;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final row = line.row;
    final speaker = row.speakerLabel == null
        ? null
        : _friendlyRuntimeLabel(row.speakerLabel!, fallback: '');
    final displayLabel = _friendlyRuntimeLabel(
      row.displayLabel,
      fallback: copy.dialogLineLabel(line.displayOrder),
    );
    final metadata = <String>[
      if (speaker?.isNotEmpty == true) speaker!,
      copy.textLanguageCount(row.authoredLocales.length),
      copy.voiceTakeCount(row.voiceTakeCount),
      if (row.selectedVoiceTakeCount > 0)
        copy.selectedVoiceCount(row.selectedVoiceTakeCount),
      if (line.isSharedAcrossQuests)
        copy.sharedQuestCount(line.linkedQuestCount),
    ];
    return Material(
      key: Key('revision3-quest-journey-dialog-line-${line.transcriptIndex}'),
      color: Theme.of(context).colorScheme.surface,
      borderRadius: BorderRadius.circular(10),
      clipBehavior: Clip.antiAlias,
      child: ListTile(
        dense: true,
        enabled: enabled,
        onTap: enabled ? onTap : null,
        title: Text(displayLabel),
        subtitle: Text(metadata.join(' · ')),
        trailing: busy
            ? const SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : interactive
            ? Tooltip(
                message: copy.openDialogLineTooltip,
                child: const Icon(Icons.chevron_right),
              )
            : null,
      ),
    );
  }
}

final class _SmallBadge extends StatelessWidget {
  const _SmallBadge({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 14),
        const SizedBox(width: 4),
        Flexible(
          child: Text(label, style: Theme.of(context).textTheme.labelSmall),
        ),
      ],
    ),
  );
}

final class _InlineNotice extends StatelessWidget {
  const _InlineNotice({required this.text, super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.all(10),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(10),
    ),
    child: Text(text),
  );
}

final class _UnavailableJourney extends StatelessWidget {
  const _UnavailableJourney({
    required this.copy,
    required this.busyAction,
    required this.actionError,
    required this.editDisabledReason,
    required this.editNameObjectivesDisabledReason,
    required this.editDescriptionConnectionsDisabledReason,
    required this.editStatesTransitionsDisabledReason,
    required this.onRetry,
    required this.onEditNameObjectives,
    required this.onEditDescriptionConnections,
    required this.onEditStatesTransitions,
  });

  final Revision3QuestJourneyPanelCopy copy;
  final String? busyAction;
  final String? actionError;
  final String? editDisabledReason;
  final String? editNameObjectivesDisabledReason;
  final String? editDescriptionConnectionsDisabledReason;
  final String? editStatesTransitionsDisabledReason;
  final VoidCallback? onRetry;
  final VoidCallback? onEditNameObjectives;
  final VoidCallback? onEditDescriptionConnections;
  final VoidCallback? onEditStatesTransitions;

  @override
  Widget build(BuildContext context) => Material(
    key: const Key('revision3-quest-journey-unavailable'),
    color: Theme.of(context).colorScheme.surface,
    child: SingleChildScrollView(
      key: const Key('revision3-quest-journey-unavailable-scroll'),
      padding: const EdgeInsets.all(24),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              const Icon(Icons.sync_problem_outlined, size: 40),
              const SizedBox(height: 12),
              Text(
                copy.unavailableTitle,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 6),
              Text(copy.unavailableBody, textAlign: TextAlign.center),
              if (busyAction != null) ...<Widget>[
                const SizedBox(height: 10),
                _JourneyBusyStatus(copy: copy),
              ],
              if (actionError != null) ...<Widget>[
                const SizedBox(height: 10),
                Text(
                  actionError!,
                  key: const Key('revision3-quest-journey-action-error'),
                  textAlign: TextAlign.center,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              if (onEditNameObjectives != null ||
                  onEditDescriptionConnections != null ||
                  onEditStatesTransitions != null ||
                  _nonEmptyReason(editDisabledReason) != null ||
                  _nonEmptyReason(editNameObjectivesDisabledReason) != null ||
                  _nonEmptyReason(editDescriptionConnectionsDisabledReason) !=
                      null ||
                  _nonEmptyReason(editStatesTransitionsDisabledReason) !=
                      null) ...<Widget>[
                const SizedBox(height: 14),
                _JourneyEditActions(
                  copy: copy,
                  busyAction: busyAction,
                  editDisabledReason: editDisabledReason,
                  editNameObjectivesDisabledReason:
                      editNameObjectivesDisabledReason,
                  editDescriptionConnectionsDisabledReason:
                      editDescriptionConnectionsDisabledReason,
                  editStatesTransitionsDisabledReason:
                      editStatesTransitionsDisabledReason,
                  onEditNameObjectives: onEditNameObjectives,
                  onEditDescriptionConnections: onEditDescriptionConnections,
                  onEditStatesTransitions: onEditStatesTransitions,
                  center: true,
                  visibleReasonExclusions: <String>{
                    ?_nonEmptyReason(copy.unavailableBody),
                  },
                ),
              ],
              if (onRetry != null) ...<Widget>[
                const SizedBox(height: 14),
                FilledButton.icon(
                  key: const Key('revision3-quest-journey-retry'),
                  onPressed: busyAction == null ? onRetry : null,
                  icon: busyAction == 'retry'
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.refresh),
                  label: Text(copy.retryLabel),
                ),
              ],
            ],
          ),
        ),
      ),
    ),
  );
}

String _transitionSummary(
  AuthoringRevision3QuestTransitionV1? transition,
  Revision3QuestJourneyPanelCopy copy,
) {
  if (transition == null) return copy.notUsed;
  final ruleCount = transition.predicate?.anyOf.length ?? 0;
  var summary = switch ((transition.externalAllowed, ruleCount)) {
    (true, 0) => copy.directTrigger,
    (true, final count) => copy.directOrAutomaticRules(count),
    (false, final count) => copy.automaticRules(count),
  };
  final followUpCount = transition.effects.length;
  if (followUpCount > 0 || transition.succeedsParent) {
    summary =
        '$summary · ${copy.followUps(followUpCount, transition.succeedsParent)}';
  }
  return summary;
}

String _friendlyRuntimeLabel(String raw, {required String fallback}) {
  var value = raw.trim();
  if (value.isEmpty ||
      value.length > 128 ||
      value.contains('/') ||
      value.contains('\\') ||
      value.contains('::') ||
      RegExp(r'[0-9a-f]{24,}', caseSensitive: false).hasMatch(value)) {
    return fallback;
  }
  value = value.replaceFirst(
    RegExp(r'^(?:UQuest|UG1RQuest|Quest|BP|U)[_\-.]*', caseSensitive: false),
    '',
  );
  final rawParts = value.split(RegExp(r'[_\-.\s]+'));
  final useful = <String>[];
  for (final part in rawParts) {
    if (part.isEmpty || RegExp(r'^\d+$').hasMatch(part)) continue;
    if (RegExp(r'^[A-Z0-9]{2,5}$').hasMatch(part)) continue;
    var friendly = part
        .replaceAllMapped(
          RegExp(r'([A-Z]+)([A-Z][a-z])'),
          (match) => '${match[1]} ${match[2]}',
        )
        .replaceAllMapped(
          RegExp(r'([a-z])([A-Z])'),
          (match) => '${match[1]} ${match[2]}',
        )
        .replaceAllMapped(
          RegExp(r'([A-Za-z])(\d)'),
          (match) => '${match[1]} ${match[2]}',
        );
    friendly = friendly.trim();
    if (friendly.isNotEmpty) useful.add(friendly);
  }
  final result = useful.join(' ').trim();
  return result.isEmpty ||
          result.length > 72 ||
          !RegExp('[a-z]').hasMatch(result)
      ? fallback
      : result;
}

String _englishObjectiveLabel(int ordinal) => 'Objective $ordinal';

String _englishAutomaticRules(int count) =>
    '$count automatic ${count == 1 ? 'rule' : 'rules'}';

String _englishDirectOrAutomaticRules(int count) =>
    'Direct trigger or $count automatic ${count == 1 ? 'rule' : 'rules'}';

String _englishFollowUps(int count, bool completesParent) {
  final parts = <String>[
    if (count > 0) '$count follow-up ${count == 1 ? 'action' : 'actions'}',
    if (completesParent) 'completes parent Quest',
  ];
  return parts.join(' + ');
}

String _englishShowDialogLines(int count) =>
    'Show $count dialog ${count == 1 ? 'line' : 'lines'}';

String _englishTextLanguageCount(int count) =>
    '$count text ${count == 1 ? 'language' : 'languages'}';

String _englishVoiceTakeCount(int count) =>
    '$count Voice ${count == 1 ? 'take' : 'takes'}';

String _englishSelectedVoiceCount(int count) =>
    '$count selected ${count == 1 ? 'take' : 'takes'}';

String _englishSharedQuestCount(int count) => 'Used by $count Quests';

String _englishDialogLineLabel(int ordinal) => 'Dialog line $ordinal';

String _germanObjectiveLabel(int ordinal) => 'Ziel $ordinal';

String _germanAutomaticRules(int count) =>
    '$count automatische ${count == 1 ? 'Regel' : 'Regeln'}';

String _germanDirectOrAutomaticRules(int count) =>
    'Direkt oder durch $count automatische ${count == 1 ? 'Regel' : 'Regeln'}';

String _germanFollowUps(int count, bool completesParent) {
  final parts = <String>[
    if (count > 0) '$count ${count == 1 ? 'Folgeaktion' : 'Folgeaktionen'}',
    if (completesParent) 'schließt die übergeordnete Quest ab',
  ];
  return parts.join(' + ');
}

String _germanShowDialogLines(int count) =>
    '$count ${count == 1 ? 'Dialogzeile' : 'Dialogzeilen'} anzeigen';

String _germanTextLanguageCount(int count) =>
    'Text in $count ${count == 1 ? 'Sprache' : 'Sprachen'}';

String _germanVoiceTakeCount(int count) =>
    '$count ${count == 1 ? 'Sprachaufnahme' : 'Sprachaufnahmen'}';

String _germanSelectedVoiceCount(int count) =>
    '$count ausgewählte ${count == 1 ? 'Aufnahme' : 'Aufnahmen'}';

String _germanSharedQuestCount(int count) =>
    'Von $count ${count == 1 ? 'Quest' : 'Quests'} verwendet';

String _germanDialogLineLabel(int ordinal) => 'Dialogzeile $ordinal';
