import 'revision3_content_index.dart';
import 'revision3_dialog_localization_authoring.dart';
import 'revision3_voice_authoring.dart';

/// The two evidence-backed work-item kinds supported by the first Voice queue.
enum Revision3VoiceProductionQueueItemKind { missingLanguage, voiceSlot }

/// The next bounded authoring decision for one queue item.
///
/// This deliberately stops at [productionDecisionsComplete]. It grants no
/// build, deployment, runtime, or general project-readiness authority.
enum Revision3VoiceProductionNextStep {
  addLanguage,
  addRecording,
  reviewAndApprove,
  selectOrRepair,
  resolveTarget,
  productionDecisionsComplete,
}

enum Revision3VoiceProductionSelectionState {
  none,
  selectedNotApproved,
  selectedApproved,
}

/// A pair of catalogs did not describe the same public project checkpoint.
final class Revision3VoiceProductionQueueCheckpointMismatch
    implements Exception {
  const Revision3VoiceProductionQueueCheckpointMismatch();
}

/// Pure next-step facts shared by the queue and the selected-line summary.
final class Revision3VoiceProductionDecision {
  const Revision3VoiceProductionDecision._({
    required this.nextStep,
    required this.selectionState,
    required this.approvedCount,
    required this.unreviewedAlternativeCount,
  });

  final Revision3VoiceProductionNextStep nextStep;
  final Revision3VoiceProductionSelectionState selectionState;
  final int approvedCount;

  /// Draft or Recorded alternatives other than the current selection.
  ///
  /// These are an optional review backlog. They never regress an already
  /// Approved, selected, and target-resolved slot.
  final int unreviewedAlternativeCount;

  bool get hasUnreviewedAlternatives => unreviewedAlternativeCount != 0;
  bool get productionDecisionsComplete =>
      nextStep == Revision3VoiceProductionNextStep.productionDecisionsComplete;
}

/// Derives a truthful Voice-production next step from one intact existing
/// slot. Precedence is intentional and must remain aligned with author UX.
Revision3VoiceProductionDecision revision3VoiceProductionDecisionFor(
  Revision3VoiceExistingSlotSummary summary,
) {
  final selectedTake = summary.selectedTakeId == null
      ? null
      : summary.candidate(summary.selectedTakeId!);
  final approvedCount = summary.candidates
      .where((take) => take.isApproved)
      .length;
  final selectionState = selectedTake == null
      ? Revision3VoiceProductionSelectionState.none
      : selectedTake.isApproved
      ? Revision3VoiceProductionSelectionState.selectedApproved
      : Revision3VoiceProductionSelectionState.selectedNotApproved;
  final unreviewedAlternativeCount = summary.candidates.where((take) {
    if (take.id == summary.selectedTakeId) return false;
    return take.status == Revision3ContentVoiceTakeStatus.draft ||
        take.status == Revision3ContentVoiceTakeStatus.recorded;
  }).length;

  final nextStep = summary.candidateCount == 0
      ? Revision3VoiceProductionNextStep.addRecording
      : approvedCount == 0
      ? Revision3VoiceProductionNextStep.reviewAndApprove
      : selectionState !=
            Revision3VoiceProductionSelectionState.selectedApproved
      ? Revision3VoiceProductionNextStep.selectOrRepair
      : summary.targetResolution !=
            Revision3ContentVoiceTargetResolution.resolved
      ? Revision3VoiceProductionNextStep.resolveTarget
      : Revision3VoiceProductionNextStep.productionDecisionsComplete;

  return Revision3VoiceProductionDecision._(
    nextStep: nextStep,
    selectionState: selectionState,
    approvedCount: approvedCount,
    unreviewedAlternativeCount: unreviewedAlternativeCount,
  );
}

sealed class Revision3VoiceProductionQueueItem {
  const Revision3VoiceProductionQueueItem({
    required this.key,
    required this.locale,
    required this.displayLabel,
    required this.nextStep,
  });

  /// Opaque widget/focus identity. It is not mutation authority.
  final String key;
  final String locale;
  final String displayLabel;
  final Revision3VoiceProductionNextStep nextStep;

  Revision3VoiceProductionQueueItemKind get kind;

  bool get isActionable =>
      nextStep != Revision3VoiceProductionNextStep.productionDecisionsComplete;
}

/// One safely editable LocalizationEntry that lacks a project authoring
/// locale. This means "language not added", not "translation is blank".
final class Revision3VoiceMissingLanguageQueueItem
    extends Revision3VoiceProductionQueueItem {
  const Revision3VoiceMissingLanguageQueueItem._({
    required this.choiceStableKey,
    required super.key,
    required super.locale,
    required super.displayLabel,
  }) : super(nextStep: Revision3VoiceProductionNextStep.addLanguage);

  final String choiceStableKey;

  @override
  Revision3VoiceProductionQueueItemKind get kind =>
      Revision3VoiceProductionQueueItemKind.missingLanguage;
}

/// One intact, already-existing VoiceSlot for an exact dialog line and locale.
final class Revision3VoiceSlotQueueItem
    extends Revision3VoiceProductionQueueItem {
  const Revision3VoiceSlotQueueItem._({
    required this.lineId,
    required this.localizationStableKey,
    required this.candidateCount,
    required this.approvedCount,
    required this.selectionState,
    required this.targetResolution,
    required this.unreviewedAlternativeCount,
    required super.key,
    required super.locale,
    required super.displayLabel,
    required super.nextStep,
  });

  /// Exact Voice action context; the host must still revalidate its checkpoint.
  final String lineId;

  /// Optional text-selection context. Voice actions do not depend on it.
  final String? localizationStableKey;
  final int candidateCount;
  final int approvedCount;
  final Revision3VoiceProductionSelectionState selectionState;
  final Revision3ContentVoiceTargetResolution targetResolution;
  final int unreviewedAlternativeCount;

  bool get hasLocalizationContext => localizationStableKey != null;
  bool get hasUnreviewedAlternatives => unreviewedAlternativeCount != 0;
  bool get productionDecisionsComplete =>
      nextStep == Revision3VoiceProductionNextStep.productionDecisionsComplete;

  @override
  Revision3VoiceProductionQueueItemKind get kind =>
      Revision3VoiceProductionQueueItemKind.voiceSlot;
}

/// Bounded, presentation-safe work list derived from exact project catalogs.
///
/// The catalog loaders and host retain checkpoint authority. This projection
/// verifies their public project ID/revision agreement, never invents work for
/// an absent VoiceSlot, and retains at most [maxItems] concrete rows.
final class Revision3VoiceProductionQueue {
  const Revision3VoiceProductionQueue._({
    required this.projectId,
    required this.projectRevision,
    required this.voiceCatalogAvailable,
    required this.items,
    required this.totalItemCount,
    required this.missingLanguageCount,
    required this.voiceSlotCount,
    required this.unboundVoiceSlotCount,
    required this._countsByNextStep,
  });

  static const int defaultMaxItems = 500;
  static const int maximumMaxItems = 5000;

  factory Revision3VoiceProductionQueue.fromCatalogs({
    required Revision3DialogLocalizationEditCatalog localizationCatalog,
    required Revision3VoiceCatalog? voiceCatalog,
    int maxItems = defaultMaxItems,
  }) {
    if (maxItems < 0 || maxItems > maximumMaxItems) {
      throw RangeError.range(maxItems, 0, maximumMaxItems, 'maxItems');
    }
    if (voiceCatalog != null &&
        (voiceCatalog.projectId != localizationCatalog.projectId ||
            voiceCatalog.projectRevision !=
                localizationCatalog.projectRevision)) {
      throw const Revision3VoiceProductionQueueCheckpointMismatch();
    }

    final counts = <Revision3VoiceProductionNextStep, int>{
      for (final step in Revision3VoiceProductionNextStep.values) step: 0,
    };
    var missingLanguageCount = 0;
    for (final choice in localizationCatalog.choices) {
      final authored = choice.locales.toSet();
      for (final locale in localizationCatalog.authoringLocales) {
        if (authored.contains(locale)) continue;
        missingLanguageCount++;
      }
    }
    counts[Revision3VoiceProductionNextStep.addLanguage] = missingLanguageCount;

    var voiceSlotCount = 0;
    var unboundVoiceSlotCount = 0;
    if (voiceCatalog != null) {
      for (final line in voiceCatalog.lines) {
        final choice = localizationCatalog.choiceForLocalizationId(
          line.localizationId,
        );
        for (final locale in line.existingSlotLocales) {
          final summary = line.slotSummaryForLocale(locale);
          if (summary == null) continue;
          voiceSlotCount++;
          if (choice == null) unboundVoiceSlotCount++;
          final decision = revision3VoiceProductionDecisionFor(summary);
          counts.update(decision.nextStep, (count) => count + 1);
        }
      }
    }

    final retained = <Revision3VoiceProductionQueueItem>[];
    if (maxItems != 0) {
      final languageItems = _missingLanguageItems(localizationCatalog).iterator;
      final voiceItems = _voiceSlotItems(
        localizationCatalog: localizationCatalog,
        voiceCatalog: voiceCatalog,
        productionDecisionsComplete: false,
      ).iterator;
      var languageHasNext = languageItems.moveNext();
      var voiceHasNext = voiceItems.moveNext();
      var preferVoice = true;
      while (retained.length < maxItems && (languageHasNext || voiceHasNext)) {
        if (preferVoice && voiceHasNext) {
          retained.add(voiceItems.current);
          voiceHasNext = voiceItems.moveNext();
        } else if (!preferVoice && languageHasNext) {
          retained.add(languageItems.current);
          languageHasNext = languageItems.moveNext();
        } else if (voiceHasNext) {
          retained.add(voiceItems.current);
          voiceHasNext = voiceItems.moveNext();
        } else {
          retained.add(languageItems.current);
          languageHasNext = languageItems.moveNext();
        }
        preferVoice = !preferVoice;
      }
      if (retained.length < maxItems) {
        final completeVoiceItems = _voiceSlotItems(
          localizationCatalog: localizationCatalog,
          voiceCatalog: voiceCatalog,
          productionDecisionsComplete: true,
        );
        for (final item in completeVoiceItems) {
          if (retained.length == maxItems) break;
          retained.add(item);
        }
      }
    }

    return Revision3VoiceProductionQueue._(
      projectId: localizationCatalog.projectId,
      projectRevision: localizationCatalog.projectRevision,
      voiceCatalogAvailable: voiceCatalog != null,
      items: List<Revision3VoiceProductionQueueItem>.unmodifiable(retained),
      totalItemCount: missingLanguageCount + voiceSlotCount,
      missingLanguageCount: missingLanguageCount,
      voiceSlotCount: voiceSlotCount,
      unboundVoiceSlotCount: unboundVoiceSlotCount,
      countsByNextStep: Map<Revision3VoiceProductionNextStep, int>.unmodifiable(
        counts,
      ),
    );
  }

  final String projectId;
  final int projectRevision;

  /// False means language work is known, but Voice-slot work was not verified.
  final bool voiceCatalogAvailable;
  final List<Revision3VoiceProductionQueueItem> items;
  final int totalItemCount;
  final int missingLanguageCount;
  final int voiceSlotCount;
  final int unboundVoiceSlotCount;
  final Map<Revision3VoiceProductionNextStep, int> _countsByNextStep;

  int get omittedItemCount => totalItemCount - items.length;
  bool get isPartial => omittedItemCount != 0;
  int get actionableCount =>
      totalItemCount -
      countFor(Revision3VoiceProductionNextStep.productionDecisionsComplete);
  int get productionDecisionsCompleteCount =>
      countFor(Revision3VoiceProductionNextStep.productionDecisionsComplete);

  int countFor(Revision3VoiceProductionNextStep step) =>
      _countsByNextStep[step] ?? 0;
}

Iterable<Revision3VoiceMissingLanguageQueueItem> _missingLanguageItems(
  Revision3DialogLocalizationEditCatalog catalog,
) sync* {
  for (final choice in catalog.choices) {
    final authored = choice.locales.toSet();
    for (final locale in catalog.authoringLocales) {
      if (authored.contains(locale)) continue;
      yield Revision3VoiceMissingLanguageQueueItem._(
        choiceStableKey: choice.stableKey,
        key: 'language:${choice.stableKey}:$locale',
        locale: locale,
        displayLabel: choice.displayLabel,
      );
    }
  }
}

Iterable<Revision3VoiceSlotQueueItem> _voiceSlotItems({
  required Revision3DialogLocalizationEditCatalog localizationCatalog,
  required Revision3VoiceCatalog? voiceCatalog,
  required bool productionDecisionsComplete,
}) sync* {
  if (voiceCatalog == null) return;
  for (final line in voiceCatalog.lines) {
    final localizationChoice = localizationCatalog.choiceForLocalizationId(
      line.localizationId,
    );
    for (final locale in line.existingSlotLocales) {
      final summary = line.slotSummaryForLocale(locale);
      if (summary == null) continue;
      final decision = revision3VoiceProductionDecisionFor(summary);
      if (decision.productionDecisionsComplete != productionDecisionsComplete) {
        continue;
      }
      yield Revision3VoiceSlotQueueItem._(
        lineId: line.lineId,
        localizationStableKey: localizationChoice?.stableKey,
        candidateCount: summary.candidateCount,
        approvedCount: decision.approvedCount,
        selectionState: decision.selectionState,
        targetResolution: summary.targetResolution,
        unreviewedAlternativeCount: decision.unreviewedAlternativeCount,
        key: 'voice:${line.lineId}:$locale',
        locale: locale,
        displayLabel: line.displayLabel,
        nextStep: decision.nextStep,
      );
    }
  }
}
