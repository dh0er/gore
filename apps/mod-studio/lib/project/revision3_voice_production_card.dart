import 'package:flutter/material.dart';

import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_production_queue.dart';

/// Author-facing copy for [Revision3VoiceProductionCard].
///
/// The card owns presentation only. Its host remains responsible for loading
/// an exact-current Voice catalog and for fail-closed action gating.
@immutable
final class Revision3VoiceProductionCardCopy {
  const Revision3VoiceProductionCardCopy({
    required this.title,
    required this.languageTemplate,
    required this.loadingTitle,
    required this.loadingDescription,
    required this.errorTitle,
    required this.errorDescription,
    required this.unavailableTitle,
    required this.unavailableDescription,
    required this.noSlotTitle,
    required this.noSlotDescription,
    required this.intactTitle,
    required this.unsafeTitle,
    required this.unsafeDescription,
    required this.oneTakeTemplate,
    required this.manyTakesTemplate,
    required this.oneApprovedTemplate,
    required this.manyApprovedTemplate,
    required this.currentSelectionLabel,
    required this.noCurrentSelectionLabel,
    required this.targetTemplate,
    required this.targetUnresolvedLabel,
    required this.targetAmbiguousLabel,
    required this.targetResolvedLabel,
    required this.nextStepLabel,
    required this.nextNoSlot,
    required this.nextNoTakes,
    required this.nextApproveTake,
    required this.nextSelectApprovedTake,
    required this.nextRepairSelection,
    required this.nextResolveTarget,
    required this.nextProductionDecisionsComplete,
    required this.statusDraftLabel,
    required this.statusRecordedLabel,
    required this.statusReviewedLabel,
    required this.statusApprovedLabel,
    required this.addTakeLabel,
    required this.planRecordingLabel,
    required this.manageTakesLabel,
    required this.resolveTargetLabel,
  });

  static const english = Revision3VoiceProductionCardCopy(
    title: 'Voice production',
    languageTemplate: 'Language: {locale}',
    loadingTitle: 'Checking Voice context',
    loadingDescription:
        'Verifying the exact dialog line and language from the current project.',
    errorTitle: 'Voice context unavailable',
    errorDescription:
        'The exact current Voice details could not be verified. Refresh the project before continuing.',
    unavailableTitle: 'Select a dialog line and language',
    unavailableDescription:
        'Voice details and actions appear after an exact line and language are selected.',
    noSlotTitle: 'No Voice setup yet',
    noSlotDescription:
        'This dialog line has no Voice setup for the selected language.',
    intactTitle: 'Voice setup',
    unsafeTitle: 'Voice setup needs attention',
    unsafeDescription:
        'The project expects a Voice setup here, but its exact safe details are unavailable. Refresh or review project problems before editing it.',
    oneTakeTemplate: '{count} take',
    manyTakesTemplate: '{count} takes',
    oneApprovedTemplate: '{count} Approved',
    manyApprovedTemplate: '{count} Approved',
    currentSelectionLabel: 'Current selection',
    noCurrentSelectionLabel: 'No take selected',
    targetTemplate: 'Target: {state}',
    targetUnresolvedLabel: 'Unresolved',
    targetAmbiguousLabel: 'Ambiguous',
    targetResolvedLabel: 'Resolved',
    nextStepLabel: 'Next step',
    nextNoSlot:
        'Plan this recording to add the language to the Work list, or add a finished take now.',
    nextNoTakes: 'Add the first recording for this language.',
    nextApproveTake:
        'Review a take, mark it Approved, and then select it in Manage takes.',
    nextSelectApprovedTake: 'Choose an Approved recording in Manage takes.',
    nextRepairSelection:
        'The selected recording is not Approved. Approve it or clear the selection in Manage takes.',
    nextResolveTarget:
        'Resolve the installed Voice target for this dialog line and language.',
    nextProductionDecisionsComplete:
        'Voice production decisions are complete. Validate & Test remains a separate project check.',
    statusDraftLabel: 'Draft',
    statusRecordedLabel: 'Recorded',
    statusReviewedLabel: 'Reviewed',
    statusApprovedLabel: 'Approved',
    addTakeLabel: 'Add take',
    planRecordingLabel: 'Plan recording',
    manageTakesLabel: 'Manage takes',
    resolveTargetLabel: 'Resolve target',
  );

  static const german = Revision3VoiceProductionCardCopy(
    title: 'Voice-Produktion',
    languageTemplate: 'Sprache: {locale}',
    loadingTitle: 'Voice-Kontext wird geprüft',
    loadingDescription:
        'Die genaue Dialogzeile und Sprache werden mit dem aktuellen Projekt abgeglichen.',
    errorTitle: 'Voice-Kontext nicht verfügbar',
    errorDescription:
        'Die aktuellen Voice-Details konnten nicht sicher geprüft werden. Aktualisiere das Projekt, bevor du fortfährst.',
    unavailableTitle: 'Dialogzeile und Sprache auswählen',
    unavailableDescription:
        'Voice-Details und Aktionen erscheinen nach einer eindeutigen Auswahl.',
    noSlotTitle: 'Noch kein Voice-Setup',
    noSlotDescription:
        'Für diese Dialogzeile gibt es in der ausgewählten Sprache noch kein Voice-Setup.',
    intactTitle: 'Voice-Setup',
    unsafeTitle: 'Voice-Setup muss geprüft werden',
    unsafeDescription:
        'Das Projekt erwartet hier ein Voice-Setup, aber dessen sichere Details sind nicht verfügbar. Aktualisiere das Projekt oder prüfe die Projektprobleme.',
    oneTakeTemplate: '{count} Aufnahme',
    manyTakesTemplate: '{count} Aufnahmen',
    oneApprovedTemplate: '{count} freigegeben',
    manyApprovedTemplate: '{count} freigegeben',
    currentSelectionLabel: 'Aktuelle Auswahl',
    noCurrentSelectionLabel: 'Keine Aufnahme ausgewählt',
    targetTemplate: 'Ziel: {state}',
    targetUnresolvedLabel: 'Nicht aufgelöst',
    targetAmbiguousLabel: 'Mehrdeutig',
    targetResolvedLabel: 'Aufgelöst',
    nextStepLabel: 'Nächster Schritt',
    nextNoSlot:
        'Plane diese Aufnahme für die Arbeitsliste ein oder füge jetzt eine fertige Aufnahme hinzu.',
    nextNoTakes: 'Füge die erste Aufnahme für diese Sprache hinzu.',
    nextApproveTake:
        'Prüfe eine Aufnahme, gib sie frei und wähle sie anschließend unter „Aufnahmen verwalten“ aus.',
    nextSelectApprovedTake:
        'Wähle unter „Aufnahmen verwalten“ eine freigegebene Aufnahme aus.',
    nextRepairSelection:
        'Die ausgewählte Aufnahme ist nicht freigegeben. Gib sie frei oder lösche die Auswahl unter „Aufnahmen verwalten“.',
    nextResolveTarget:
        'Löse das installierte Voice-Ziel für diese Dialogzeile und Sprache auf.',
    nextProductionDecisionsComplete:
        'Die Voice-Produktionsentscheidungen sind abgeschlossen. „Validieren & Testen“ bleibt eine separate Projektprüfung.',
    statusDraftLabel: 'Entwurf',
    statusRecordedLabel: 'Aufgenommen',
    statusReviewedLabel: 'Geprüft',
    statusApprovedLabel: 'Freigegeben',
    addTakeLabel: 'Aufnahme hinzufügen',
    planRecordingLabel: 'Aufnahme einplanen',
    manageTakesLabel: 'Aufnahmen verwalten',
    resolveTargetLabel: 'Ziel auflösen',
  );

  final String title;
  final String languageTemplate;
  final String loadingTitle;
  final String loadingDescription;
  final String errorTitle;
  final String errorDescription;
  final String unavailableTitle;
  final String unavailableDescription;
  final String noSlotTitle;
  final String noSlotDescription;
  final String intactTitle;
  final String unsafeTitle;
  final String unsafeDescription;
  final String oneTakeTemplate;
  final String manyTakesTemplate;
  final String oneApprovedTemplate;
  final String manyApprovedTemplate;
  final String currentSelectionLabel;
  final String noCurrentSelectionLabel;
  final String targetTemplate;
  final String targetUnresolvedLabel;
  final String targetAmbiguousLabel;
  final String targetResolvedLabel;
  final String nextStepLabel;
  final String nextNoSlot;
  final String nextNoTakes;
  final String nextApproveTake;
  final String nextSelectApprovedTake;
  final String nextRepairSelection;
  final String nextResolveTarget;
  final String nextProductionDecisionsComplete;
  final String statusDraftLabel;
  final String statusRecordedLabel;
  final String statusReviewedLabel;
  final String statusApprovedLabel;
  final String addTakeLabel;
  final String planRecordingLabel;
  final String manageTakesLabel;
  final String resolveTargetLabel;

  String language(String locale) =>
      languageTemplate.replaceAll('{locale}', locale);

  String takeCount(int count) =>
      (count == 1 ? oneTakeTemplate : manyTakesTemplate).replaceAll(
        '{count}',
        '$count',
      );

  String approvedCount(int count) =>
      (count == 1 ? oneApprovedTemplate : manyApprovedTemplate).replaceAll(
        '{count}',
        '$count',
      );

  String target(String state) => targetTemplate.replaceAll('{state}', state);
}

/// Compact, always-visible summary for one exact dialog-line/language Voice
/// production context.
///
/// Technical IDs, paths, preview authority, build authority, and mutation
/// authority deliberately remain outside this widget. A non-null action means
/// the host already proved that exact action safe for the current checkpoint.
class Revision3VoiceProductionCard extends StatelessWidget {
  const Revision3VoiceProductionCard({
    required this.line,
    required this.locale,
    required this.slotExpected,
    this.projectionRejected = false,
    this.loading = false,
    this.error,
    this.onAddTake,
    this.onPlanRecording,
    this.onManageTakes,
    this.onResolveTarget,
    this.copy = Revision3VoiceProductionCardCopy.english,
    super.key,
  });

  final Revision3VoiceDialogLineChoice? line;
  final String? locale;
  final bool slotExpected;

  /// The host selected an exact line/language context, but the current strict
  /// Voice projection omitted that line. This is unsafe, not an absent choice.
  final bool projectionRejected;
  final bool loading;

  /// Only the presence of this value is presented. Its potentially technical
  /// message is intentionally never rendered.
  final Object? error;

  final VoidCallback? onAddTake;
  final VoidCallback? onPlanRecording;
  final VoidCallback? onManageTakes;
  final VoidCallback? onResolveTarget;
  final Revision3VoiceProductionCardCopy copy;

  @override
  Widget build(BuildContext context) {
    final selectedLine = line;
    final selectedLocale = locale;
    final summary = selectedLine == null || selectedLocale == null
        ? null
        : selectedLine.slotSummaryForLocale(selectedLocale);
    final state = _stateFor(
      loading: loading,
      error: error,
      line: selectedLine,
      locale: selectedLocale,
      slotExpected: slotExpected,
      projectionRejected: projectionRejected,
      summary: summary,
    );
    final theme = Theme.of(context);

    return Card(
      key: const Key('revision3-voice-production-card'),
      margin: EdgeInsets.zero,
      child: Semantics(
        container: true,
        label: copy.title,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.record_voice_over_outlined,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(copy.title, style: theme.textTheme.titleMedium),
                        if (selectedLine != null && selectedLocale != null) ...[
                          const SizedBox(height: 3),
                          Wrap(
                            spacing: 8,
                            runSpacing: 4,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              Text(
                                selectedLine.displayLabel,
                                key: const Key(
                                  'revision3-voice-production-line-label',
                                ),
                                style: theme.textTheme.bodyMedium,
                              ),
                              Chip(
                                key: const Key(
                                  'revision3-voice-production-locale',
                                ),
                                visualDensity: VisualDensity.compact,
                                label: Text(copy.language(selectedLocale)),
                              ),
                            ],
                          ),
                        ],
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              switch (state) {
                _VoiceProductionCardState.loading => _simpleState(
                  key: const Key('revision3-voice-production-loading'),
                  context: context,
                  icon: Icons.sync,
                  title: copy.loadingTitle,
                  description: copy.loadingDescription,
                  progress: true,
                ),
                _VoiceProductionCardState.error => _simpleState(
                  key: const Key('revision3-voice-production-error'),
                  context: context,
                  icon: Icons.error_outline,
                  title: copy.errorTitle,
                  description: copy.errorDescription,
                  warning: true,
                ),
                _VoiceProductionCardState.unavailable => _simpleState(
                  key: const Key('revision3-voice-production-unavailable'),
                  context: context,
                  icon: Icons.touch_app_outlined,
                  title: copy.unavailableTitle,
                  description: copy.unavailableDescription,
                ),
                _VoiceProductionCardState.noSlot => _simpleState(
                  key: const Key('revision3-voice-production-no-slot'),
                  context: context,
                  icon: Icons.mic_none_outlined,
                  title: copy.noSlotTitle,
                  description: copy.noSlotDescription,
                  nextStep: copy.nextNoSlot,
                ),
                _VoiceProductionCardState.unsafe => _simpleState(
                  key: const Key('revision3-voice-production-unsafe'),
                  context: context,
                  icon: Icons.report_problem_outlined,
                  title: copy.unsafeTitle,
                  description: copy.unsafeDescription,
                  warning: true,
                ),
                _VoiceProductionCardState.intact => _intactState(
                  context,
                  summary!,
                ),
              },
              if (_showsActions(state)) ...[
                const SizedBox(height: 12),
                _actions(state),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _simpleState({
    required Key key,
    required BuildContext context,
    required IconData icon,
    required String title,
    required String description,
    String? nextStep,
    bool progress = false,
    bool warning = false,
  }) {
    final theme = Theme.of(context);
    final color = warning
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;
    return Row(
      key: key,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (progress)
          const SizedBox.square(
            dimension: 20,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          Icon(icon, size: 20, color: color),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: theme.textTheme.titleSmall),
              const SizedBox(height: 2),
              Text(description),
              if (nextStep != null) ...[
                const SizedBox(height: 8),
                Text(copy.nextStepLabel, style: theme.textTheme.labelLarge),
                Text(nextStep),
              ],
            ],
          ),
        ),
      ],
    );
  }

  Widget _intactState(
    BuildContext context,
    Revision3VoiceExistingSlotSummary summary,
  ) {
    final theme = Theme.of(context);
    final decision = revision3VoiceProductionDecisionFor(summary);
    final approvedCount = decision.approvedCount;
    final selectedTake = summary.selectedTakeId == null
        ? null
        : summary.candidate(summary.selectedTakeId!);
    final targetLabel = _targetLabel(summary.targetResolution);
    final nextStep = _nextStep(decision);

    return Column(
      key: const Key('revision3-voice-production-intact'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(copy.intactTitle, style: theme.textTheme.titleSmall),
        const SizedBox(height: 8),
        Wrap(
          key: const Key('revision3-voice-production-facts'),
          spacing: 8,
          runSpacing: 6,
          children: [
            _FactChip(label: copy.takeCount(summary.candidateCount)),
            _FactChip(label: copy.approvedCount(approvedCount)),
            _FactChip(label: copy.target(targetLabel)),
          ],
        ),
        const SizedBox(height: 10),
        Text(copy.currentSelectionLabel, style: theme.textTheme.labelLarge),
        const SizedBox(height: 3),
        if (selectedTake == null)
          Text(
            copy.noCurrentSelectionLabel,
            key: const Key('revision3-voice-production-no-selection'),
          )
        else
          Wrap(
            key: const Key('revision3-voice-production-selection'),
            spacing: 8,
            runSpacing: 4,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text(selectedTake.displayLabel),
              Chip(
                visualDensity: VisualDensity.compact,
                label: Text(_statusLabel(selectedTake.status)),
              ),
            ],
          ),
        const SizedBox(height: 10),
        Text(copy.nextStepLabel, style: theme.textTheme.labelLarge),
        const SizedBox(height: 2),
        Text(nextStep, key: const Key('revision3-voice-production-next-step')),
      ],
    );
  }

  Widget _actions(_VoiceProductionCardState state) {
    final allowAdd =
        state == _VoiceProductionCardState.noSlot ||
        state == _VoiceProductionCardState.intact;
    final allowPlan = state == _VoiceProductionCardState.noSlot;
    final allowExistingSlotActions = state == _VoiceProductionCardState.intact;
    return Wrap(
      key: const Key('revision3-voice-production-actions'),
      spacing: 8,
      runSpacing: 8,
      children: [
        if (allowAdd && onAddTake != null)
          FilledButton.tonalIcon(
            key: const Key('revision3-voice-production-add'),
            onPressed: onAddTake,
            icon: const Icon(Icons.add),
            label: Text(copy.addTakeLabel),
          ),
        if (allowPlan && onPlanRecording != null)
          OutlinedButton.icon(
            key: const Key('revision3-voice-production-plan'),
            onPressed: onPlanRecording,
            icon: const Icon(Icons.event_note_outlined),
            label: Text(copy.planRecordingLabel),
          ),
        if (allowExistingSlotActions && onManageTakes != null)
          OutlinedButton.icon(
            key: const Key('revision3-voice-production-manage'),
            onPressed: onManageTakes,
            icon: const Icon(Icons.tune),
            label: Text(copy.manageTakesLabel),
          ),
        if (allowExistingSlotActions && onResolveTarget != null)
          OutlinedButton.icon(
            key: const Key('revision3-voice-production-resolve'),
            onPressed: onResolveTarget,
            icon: const Icon(Icons.link),
            label: Text(copy.resolveTargetLabel),
          ),
      ],
    );
  }

  bool _showsActions(_VoiceProductionCardState state) => switch (state) {
    _VoiceProductionCardState.noSlot =>
      onAddTake != null || onPlanRecording != null,
    _VoiceProductionCardState.intact =>
      onAddTake != null || onManageTakes != null || onResolveTarget != null,
    _ => false,
  };

  String _targetLabel(Revision3ContentVoiceTargetResolution resolution) =>
      switch (resolution) {
        Revision3ContentVoiceTargetResolution.unresolved =>
          copy.targetUnresolvedLabel,
        Revision3ContentVoiceTargetResolution.ambiguous =>
          copy.targetAmbiguousLabel,
        Revision3ContentVoiceTargetResolution.resolved =>
          copy.targetResolvedLabel,
      };

  String _statusLabel(Revision3ContentVoiceTakeStatus status) =>
      switch (status) {
        Revision3ContentVoiceTakeStatus.draft => copy.statusDraftLabel,
        Revision3ContentVoiceTakeStatus.recorded => copy.statusRecordedLabel,
        Revision3ContentVoiceTakeStatus.reviewed => copy.statusReviewedLabel,
        Revision3ContentVoiceTakeStatus.approved => copy.statusApprovedLabel,
      };

  String _nextStep(
    Revision3VoiceProductionDecision decision,
  ) => switch (decision.nextStep) {
    // This card only presents existing VoiceSlots. The language branch is
    // unreachable, but remains exhaustive if the shared enum grows.
    Revision3VoiceProductionNextStep.addLanguage => copy.nextSelectApprovedTake,
    Revision3VoiceProductionNextStep.addRecording => copy.nextNoTakes,
    Revision3VoiceProductionNextStep.reviewAndApprove => copy.nextApproveTake,
    Revision3VoiceProductionNextStep.selectOrRepair =>
      decision.selectionState ==
              Revision3VoiceProductionSelectionState.selectedNotApproved
          ? copy.nextRepairSelection
          : copy.nextSelectApprovedTake,
    Revision3VoiceProductionNextStep.resolveTarget => copy.nextResolveTarget,
    Revision3VoiceProductionNextStep.productionDecisionsComplete =>
      copy.nextProductionDecisionsComplete,
  };
}

enum _VoiceProductionCardState {
  loading,
  error,
  unavailable,
  noSlot,
  unsafe,
  intact,
}

_VoiceProductionCardState _stateFor({
  required bool loading,
  required Object? error,
  required Revision3VoiceDialogLineChoice? line,
  required String? locale,
  required bool slotExpected,
  required bool projectionRejected,
  required Revision3VoiceExistingSlotSummary? summary,
}) {
  if (loading) return _VoiceProductionCardState.loading;
  if (error != null) return _VoiceProductionCardState.error;
  if (projectionRejected) return _VoiceProductionCardState.unsafe;
  if (line == null || locale == null) {
    return _VoiceProductionCardState.unavailable;
  }
  if (summary != null) return _VoiceProductionCardState.intact;
  return slotExpected
      ? _VoiceProductionCardState.unsafe
      : _VoiceProductionCardState.noSlot;
}

class _FactChip extends StatelessWidget {
  const _FactChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) =>
      Chip(visualDensity: VisualDensity.compact, label: Text(label));
}
