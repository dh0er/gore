import 'dart:convert';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_quest_transcript_authoring.dart';

/// Hard authoring limit inherited from the revision-3 Quest transcript model.
const revision3QuestJourneyMaxDialogLines = 256;

/// The persisted stages and conditional migration of the bounded Quest Draft
/// setup path.
///
/// These labels describe authored project facts only. They are deliberately
/// unrelated to build, deployment, runtime, or save-game readiness.
enum Revision3QuestDraftSetupStepKind {
  questDetails,
  openingDialog,
  legacyBehavior,
}

/// Exact-current, read-only progress for the persistent "Write a Quest" path.
///
/// The setup never invents a separate completion flag. Every value is derived
/// from the same exact Journey projection that the author can inspect below
/// it. Voice counts are supplemental production context and never gate a step.
final class Revision3QuestDraftSetup {
  const Revision3QuestDraftSetup._({
    required this.questDetailsComplete,
    required this.openingDialogComplete,
    required this.legacyBehaviorReviewRequired,
    required this.openingDialogLineCount,
    required this.openingTextLanguageCount,
    required this.openingVoiceTakeCount,
    required this.openingSelectedVoiceTakeCount,
  });

  /// Whether the one atomic Quest-details publication contains the required
  /// name, objectives, story/giver bindings, and valid generated behavior.
  ///
  /// Exact-current managed Quests satisfy this invariant. It remains explicit
  /// so the UI describes the persisted boundary instead of inventing separate
  /// name and connection checkpoints.
  final bool questDetailsComplete;
  final bool openingDialogComplete;

  /// Legacy generator-v2/v3 Quests expose fixed synthetic behavior. They need
  /// one explicit review/migration after their separately saved opening dialog;
  /// modern generator-v4 creation does not show a universal behavior step.
  final bool legacyBehaviorReviewRequired;
  final int openingDialogLineCount;
  final int openingTextLanguageCount;
  final int openingVoiceTakeCount;
  final int openingSelectedVoiceTakeCount;

  bool complete(Revision3QuestDraftSetupStepKind step) => switch (step) {
    Revision3QuestDraftSetupStepKind.questDetails => questDetailsComplete,
    Revision3QuestDraftSetupStepKind.openingDialog => openingDialogComplete,
    Revision3QuestDraftSetupStepKind.legacyBehavior =>
      !legacyBehaviorReviewRequired,
  };

  bool get draftSetupComplete =>
      questDetailsComplete &&
      openingDialogComplete &&
      !legacyBehaviorReviewRequired;

  /// The sole recommended continuation shown by the persistent setup path.
  ///
  /// The opening dialog is always the first separately persisted continuation.
  /// A legacy behavior migration follows only when the exact Quest still uses
  /// synthetic fixed behavior. Otherwise Dialog & Voice remains the
  /// conservative review continuation. That does not make Voice mandatory and
  /// does not turn Draft completion into runtime readiness.
  Revision3QuestDraftSetupStepKind get recommendedStep {
    if (!openingDialogComplete) {
      return Revision3QuestDraftSetupStepKind.openingDialog;
    }
    if (legacyBehaviorReviewRequired) {
      return Revision3QuestDraftSetupStepKind.legacyBehavior;
    }
    return Revision3QuestDraftSetupStepKind.openingDialog;
  }
}

/// One or more supposedly exact inputs no longer describe the same checkpoint.
///
/// The journey projection is read-only and deliberately fails closed instead
/// of trying to repair, merge, or infer identities across revisions.
final class Revision3QuestJourneyStaleCheckpointException implements Exception {
  const Revision3QuestJourneyStaleCheckpointException();
}

/// The authored lifecycle behavior for the main Quest or one objective.
///
/// These are project facts only. They do not assert that the game runtime can
/// load, execute, or persist the behavior.
final class Revision3QuestJourneyNodeBehavior {
  const Revision3QuestJourneyNodeBehavior._({
    required this.node,
    required this.availability,
    required this.start,
    required this.success,
    required this.failure,
  });

  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionV1 availability;
  final AuthoringRevision3QuestTransitionV1 start;
  final AuthoringRevision3QuestTransitionV1? success;
  final AuthoringRevision3QuestTransitionV1? failure;

  List<AuthoringRevision3QuestTransitionV1> get orderedTransitions =>
      List<AuthoringRevision3QuestTransitionV1>.unmodifiable(
        <AuthoringRevision3QuestTransitionV1>[
          availability,
          start,
          ?success,
          ?failure,
        ],
      );
}

/// One transcript row retained in its exact authored order.
final class Revision3QuestJourneyDialogLine {
  const Revision3QuestJourneyDialogLine._({
    required this.transcriptIndex,
    required this.row,
    required this.linkedQuestCount,
  });

  /// Zero-based position in the Quest's complete transcript.
  final int transcriptIndex;
  final Revision3QuestTranscriptRow row;

  /// Number of exact-current Quest drafts that reference this DialogLine.
  final int linkedQuestCount;

  int get displayOrder => transcriptIndex + 1;
  String get lineId => row.lineId;
  int? get objectiveSlot => row.objectiveSlot;
  bool get isSharedAcrossQuests => linkedQuestCount > 1;
}

/// One objective in authored presentation order with its project behavior and
/// associated transcript rows.
final class Revision3QuestJourneyObjective {
  const Revision3QuestJourneyObjective._({
    required this.title,
    required this.transitionSlot,
    required this.stableObjectiveSlot,
    required this.behavior,
    required this.dialogLines,
  });

  final String title;

  /// Slot used by the effective transition plan. For legacy generator-v2/v3
  /// Quests this is synthetic and must not be used to group transcript rows.
  final int transitionSlot;

  /// Persisted semantic objective slot. It is intentionally null for legacy
  /// generator-v2/v3 Quests, where no such authoring identity exists.
  final int? stableObjectiveSlot;

  final Revision3QuestJourneyNodeBehavior behavior;
  final List<Revision3QuestJourneyDialogLine> dialogLines;
}

/// Exact-current, read-only projection for one coherent Quest journey view.
///
/// Composition performs no localization reads, mutations, build, deploy, or
/// runtime work. The original transition and transcript objects are retained
/// so a later UI can hand off to their existing exact authoring services.
final class Revision3QuestJourneyProjection {
  const Revision3QuestJourneyProjection._({
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.technicalId,
    required this.title,
    required this.moduleNamespace,
    required this.parentRuntimeClass,
    required this.giverRuntimeUniqueName,
    required this.legacySyntheticBehavior,
    required this.rootBehavior,
    required this.objectives,
    required this.orderedDialogLines,
    required this.generalDialogLines,
  });

  factory Revision3QuestJourneyProjection.compose({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required Revision3ContentEntity module,
    required AuthoringRevision3QuestTransitionsSeed transitionSeed,
    required Revision3QuestTranscriptProjection transcript,
  }) {
    final summary = quest.summary.questDraft;
    final moduleReferences = quest.references
        .where((reference) => reference.role == 'draft_script_module')
        .toList(growable: false);
    final owner = module.origin.generatedOwner;
    _requireJourneyBinding(
      quest.kind == Revision3ContentEntityKind.questDraft &&
          summary != null &&
          quest.problemCount == 0 &&
          identical(index.entityById(quest.id), quest) &&
          module.kind == Revision3ContentEntityKind.scriptModule &&
          module.problemCount == 0 &&
          identical(index.entityById(module.id), module) &&
          moduleReferences.length == 1 &&
          moduleReferences.single.qualifier == null &&
          moduleReferences.single.resolution ==
              Revision3ContentReferenceResolution.resolved &&
          moduleReferences.single.target.projectId == index.projectId &&
          moduleReferences.single.target.entityId == module.id &&
          moduleReferences.single.target.expectedKind ==
              Revision3ContentEntityKind.scriptModule &&
          owner != null &&
          owner.projectId == index.projectId &&
          owner.entityId == quest.id &&
          owner.expectedKind == Revision3ContentEntityKind.questDraft &&
          module.origin.generatorVersion == transitionSeed.generatorVersion &&
          module.summary.primaryIdentity == summary.moduleNamespace,
    );
    final questSummary = summary!;

    _requireJourneyBinding(
      transitionSeed.projectId == index.projectId &&
          transitionSeed.projectRevision == index.projectRevision &&
          transitionSeed.questId == quest.id &&
          transitionSeed.questRevision == quest.revision &&
          transitionSeed.moduleId == module.id &&
          transitionSeed.moduleRevision == module.revision &&
          _seedTargetMatchesIndex(index, transitionSeed) &&
          _sameSeal(
            transitionSeed.transitionPlanSeal,
            transitionSeed.transitionPlan.contentSeal,
          ),
    );
    _requireObjectivesMatchSeed(questSummary, transitionSeed);

    _requireJourneyBinding(
      transcript.projectId == index.projectId &&
          transcript.projectRevision == index.projectRevision &&
          transcript.questId == quest.id &&
          transcript.questRevision == quest.revision &&
          transcript.checkpointIdentity.isNotEmpty &&
          transcript.rows.length == questSummary.transcriptCount &&
          transcript.rows.length <= revision3QuestJourneyMaxDialogLines,
    );
    _requireTranscriptObjectives(
      summary: questSummary,
      seed: transitionSeed,
      transcript: transcript,
    );

    final availableLines = <String, Revision3QuestTranscriptLineChoice>{};
    for (final line in transcript.availableLines) {
      if (availableLines.containsKey(line.lineId)) {
        throw const Revision3QuestJourneyStaleCheckpointException();
      }
      availableLines[line.lineId] = line;
    }
    final transcriptReferences = quest.references
        .where((reference) => reference.role == 'quest_transcript_line')
        .toList(growable: false);
    _requireJourneyBinding(
      transcriptReferences.length == transcript.rows.length,
    );

    final orderedDialog = <Revision3QuestJourneyDialogLine>[];
    final seenLines = <String>{};
    for (
      var indexPosition = 0;
      indexPosition < transcript.rows.length;
      indexPosition++
    ) {
      final row = transcript.rows[indexPosition];
      final reference = transcriptReferences[indexPosition];
      final referenceSlot = reference.qualifier == null
          ? null
          : int.tryParse(reference.qualifier!);
      final line = index.entityById(row.lineId);
      final available = availableLines[row.lineId];
      _requireJourneyBinding(
        seenLines.add(row.lineId) &&
            line != null &&
            line.kind == Revision3ContentEntityKind.dialogLine &&
            line.problemCount == 0 &&
            available != null &&
            identical(available, row.line) &&
            reference.qualifier ==
                (row.objectiveSlot == null ? null : '${row.objectiveSlot}') &&
            referenceSlot == row.objectiveSlot &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            reference.target.projectId == index.projectId &&
            reference.target.entityId == row.lineId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.dialogLine,
      );
      if (transitionSeed.legacySynthetic && row.objectiveSlot != null) {
        throw const Revision3QuestJourneyStaleCheckpointException();
      }
      final linkedQuestIds = <String>{
        for (final backlink in index.backlinksToEntity(row.lineId))
          if (backlink.source.kind == Revision3ContentEntityKind.questDraft &&
              backlink.reference.role == 'quest_transcript_line' &&
              backlink.reference.resolution ==
                  Revision3ContentReferenceResolution.resolved)
            backlink.source.id,
      };
      _requireJourneyBinding(linkedQuestIds.contains(quest.id));
      orderedDialog.add(
        Revision3QuestJourneyDialogLine._(
          transcriptIndex: indexPosition,
          row: row,
          linkedQuestCount: linkedQuestIds.length,
        ),
      );
    }

    final rowsByStableSlot = <int, List<Revision3QuestJourneyDialogLine>>{};
    final generalDialog = <Revision3QuestJourneyDialogLine>[];
    for (final line in orderedDialog) {
      final slot = line.objectiveSlot;
      if (slot == null) {
        generalDialog.add(line);
      } else {
        rowsByStableSlot
            .putIfAbsent(slot, () => <Revision3QuestJourneyDialogLine>[])
            .add(line);
      }
    }
    final stableSlots = transitionSeed.legacySynthetic
        ? const <int>{}
        : questSummary.objectiveSlots.toSet();
    _requireJourneyBinding(rowsByStableSlot.keys.every(stableSlots.contains));

    final plan = transitionSeed.transitionPlan;
    final objectives = <Revision3QuestJourneyObjective>[
      for (final objective in transitionSeed.objectives)
        Revision3QuestJourneyObjective._(
          title: objective.title,
          transitionSlot: objective.slot,
          stableObjectiveSlot: transitionSeed.legacySynthetic
              ? null
              : objective.slot,
          behavior: _behaviorFor(
            plan,
            AuthoringRevision3QuestTransitionNodeV1.objective(objective.slot),
          ),
          dialogLines: List<Revision3QuestJourneyDialogLine>.unmodifiable(
            transitionSeed.legacySynthetic
                ? const <Revision3QuestJourneyDialogLine>[]
                : rowsByStableSlot[objective.slot] ??
                      const <Revision3QuestJourneyDialogLine>[],
          ),
        ),
    ];

    return Revision3QuestJourneyProjection._(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      checkpointIdentity: transcript.checkpointIdentity,
      questId: quest.id,
      questRevision: quest.revision,
      moduleId: module.id,
      moduleRevision: module.revision,
      technicalId: questSummary.technicalId,
      title: questSummary.title,
      moduleNamespace: questSummary.moduleNamespace,
      parentRuntimeClass: questSummary.parentRuntimeClass,
      giverRuntimeUniqueName: questSummary.giverRuntimeUniqueName,
      legacySyntheticBehavior: transitionSeed.legacySynthetic,
      rootBehavior: _behaviorFor(
        plan,
        const AuthoringRevision3QuestTransitionNodeV1.root(),
      ),
      objectives: List<Revision3QuestJourneyObjective>.unmodifiable(objectives),
      orderedDialogLines: List<Revision3QuestJourneyDialogLine>.unmodifiable(
        orderedDialog,
      ),
      generalDialogLines: List<Revision3QuestJourneyDialogLine>.unmodifiable(
        generalDialog,
      ),
    );
  }

  final String projectId;
  final int projectRevision;

  /// Opaque exact WorkingHead identity inherited from the transcript load.
  final String checkpointIdentity;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final String technicalId;
  final String title;
  final String moduleNamespace;
  final String parentRuntimeClass;
  final String giverRuntimeUniqueName;

  /// True when the effective transition plan was synthesized from a frozen
  /// generator-v2/v3 Quest. It is project behavior, not persisted V4 slots.
  final bool legacySyntheticBehavior;

  /// Authored behavior for the main Quest/root state.
  final Revision3QuestJourneyNodeBehavior rootBehavior;
  final List<Revision3QuestJourneyObjective> objectives;

  /// Complete transcript in exact authored order.
  final List<Revision3QuestJourneyDialogLine> orderedDialogLines;

  /// Transcript rows without a persisted objective association.
  final List<Revision3QuestJourneyDialogLine> generalDialogLines;

  /// Persistent Guided-mode progress derived only from exact project facts.
  Revision3QuestDraftSetup get draftSetup {
    final opening = orderedDialogLines.isEmpty
        ? null
        : orderedDialogLines.first.row;
    return Revision3QuestDraftSetup._(
      questDetailsComplete:
          title.trim().isNotEmpty &&
          objectives.isNotEmpty &&
          objectives.every((objective) => objective.title.trim().isNotEmpty) &&
          parentRuntimeClass.trim().isNotEmpty &&
          giverRuntimeUniqueName.trim().isNotEmpty,
      openingDialogComplete:
          opening != null && opening.authoredLocales.isNotEmpty,
      legacyBehaviorReviewRequired: legacySyntheticBehavior,
      openingDialogLineCount: orderedDialogLines.length,
      openingTextLanguageCount: opening?.authoredLocales.length ?? 0,
      openingVoiceTakeCount: opening?.voiceTakeCount ?? 0,
      openingSelectedVoiceTakeCount: opening?.selectedVoiceTakeCount ?? 0,
    );
  }
}

void _requireObjectivesMatchSeed(
  Revision3ContentQuestDraftSummary summary,
  AuthoringRevision3QuestTransitionsSeed seed,
) {
  final objectives = seed.objectives;
  final titles = summary.objectiveTitles;
  final order = seed.transitionPlan.objectiveOrder;
  _requireJourneyBinding(
    objectives.length == titles.length && objectives.length == order.length,
  );
  for (var position = 0; position < objectives.length; position++) {
    _requireJourneyBinding(
      objectives[position].title == titles[position] &&
          objectives[position].slot == order[position],
    );
  }
  if (seed.legacySynthetic) {
    _requireJourneyBinding(summary.objectiveSlots.isEmpty);
    return;
  }
  _requireJourneyBinding(summary.objectiveSlots.length == objectives.length);
  for (var position = 0; position < objectives.length; position++) {
    _requireJourneyBinding(
      summary.objectiveSlots[position] == objectives[position].slot,
    );
  }
}

void _requireTranscriptObjectives({
  required Revision3ContentQuestDraftSummary summary,
  required AuthoringRevision3QuestTransitionsSeed seed,
  required Revision3QuestTranscriptProjection transcript,
}) {
  if (seed.legacySynthetic) {
    _requireJourneyBinding(
      summary.objectiveSlots.isEmpty && transcript.objectives.isEmpty,
    );
    return;
  }
  _requireJourneyBinding(
    transcript.objectives.length == seed.objectives.length,
  );
  for (var position = 0; position < seed.objectives.length; position++) {
    final expected = seed.objectives[position];
    final actual = transcript.objectives[position];
    _requireJourneyBinding(
      actual.slot == expected.slot && actual.title == expected.title,
    );
  }
}

Revision3QuestJourneyNodeBehavior _behaviorFor(
  AuthoringRevision3QuestTransitionPlanV1 plan,
  AuthoringRevision3QuestTransitionNodeV1 node,
) {
  final byEdge =
      <
        AuthoringRevision3QuestTransitionEdgeV1,
        AuthoringRevision3QuestTransitionV1
      >{};
  for (final transition in plan.transitions) {
    if (transition.node != node) continue;
    if (byEdge.putIfAbsent(transition.edge, () => transition) != transition) {
      throw const Revision3QuestJourneyStaleCheckpointException();
    }
  }
  final availability =
      byEdge[AuthoringRevision3QuestTransitionEdgeV1.availability];
  final start = byEdge[AuthoringRevision3QuestTransitionEdgeV1.start];
  _requireJourneyBinding(availability != null && start != null);
  return Revision3QuestJourneyNodeBehavior._(
    node: node,
    availability: availability!,
    start: start!,
    success: byEdge[AuthoringRevision3QuestTransitionEdgeV1.success],
    failure: byEdge[AuthoringRevision3QuestTransitionEdgeV1.failure],
  );
}

bool _seedTargetMatchesIndex(
  Revision3ContentIndex index,
  AuthoringRevision3QuestTransitionsSeed seed,
) {
  try {
    final target = jsonDecode(seed.targetCanonicalJson);
    if (target is! Map<String, dynamic> || target.length != 1) return false;
    final executable = target['executable'];
    if (executable is! Map<String, dynamic> || executable.length != 2) {
      return false;
    }
    return executable['byte_len'] == index.targetExecutableByteLength &&
        executable['sha256'] == index.targetExecutableSha256;
  } on FormatException {
    return false;
  }
}

bool _sameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

void _requireJourneyBinding(bool condition) {
  if (!condition) {
    throw const Revision3QuestJourneyStaleCheckpointException();
  }
}
