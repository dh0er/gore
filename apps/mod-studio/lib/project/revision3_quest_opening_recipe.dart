import '../core/mod_ffi.dart';
import 'current_project_controller.dart';
import 'revision3_dialog_line_authoring.dart';
import 'revision3_quest_authoring.dart';
import 'revision3_quest_transcript_authoring.dart';

typedef Revision3QuestOpeningRecipeCheckpointReader =
    Future<ManagedRevision3CurrentProjectState?> Function();

typedef Revision3QuestOpeningRecipeQuestStepAction =
    Future<Revision3QuestOpeningRecipeQuestStep?> Function({
      required ManagedRevision3CurrentProjectState expectedCheckpoint,
    });

typedef Revision3QuestOpeningRecipeLineStepAction =
    Future<Revision3QuestOpeningRecipeLineStep?> Function({
      required Revision3QuestOpeningRecipeHandoff handoff,
    });

/// The exact Quest publication together with the fully reopened managed
/// checkpoint observed by its host immediately after publication.
///
/// The recipe validates both values and re-reads the current checkpoint before
/// handing authority to the opening-line step. This wrapper does not add any
/// build, runtime, topic, or game-write authority.
final class Revision3QuestOpeningRecipeQuestStep {
  const Revision3QuestOpeningRecipeQuestStep({
    required this.publication,
    required this.checkpoint,
  });

  final Revision3QuestDraftPublication publication;
  final ManagedRevision3CurrentProjectState checkpoint;
}

/// The exact create-and-insert publication together with its fully reopened
/// managed checkpoint.
final class Revision3QuestOpeningRecipeLineStep {
  const Revision3QuestOpeningRecipeLineStep({
    required this.publication,
    required this.checkpoint,
  });

  final Revision3QuestTranscriptPublication publication;
  final ManagedRevision3CurrentProjectState checkpoint;
}

/// Exact handoff from the published Quest-only checkpoint to step two.
///
/// Hosts must bind all content reads and the transcript publication to
/// [questCheckpoint], including its root and canonical WorkingHead bytes.
final class Revision3QuestOpeningRecipeHandoff {
  const Revision3QuestOpeningRecipeHandoff._({
    required this.openingCheckpoint,
    required this.questStep,
  });

  final ManagedRevision3CurrentProjectState openingCheckpoint;
  final Revision3QuestOpeningRecipeQuestStep questStep;

  Revision3QuestDraftPublication get questPublication => questStep.publication;

  ManagedRevision3CurrentProjectState get questCheckpoint =>
      questStep.checkpoint;
}

enum Revision3QuestOpeningRecipeNoChangeReason { cancelled, failed }

enum Revision3QuestOpeningRecipeQuestOnlyReason {
  openingLineCancelled,
  openingLineFailed,
}

enum Revision3QuestOpeningRecipeLockReason {
  checkpointUnavailable,
  openingCheckpointDrift,
  questStepStale,
  questPublicationMismatch,
  questCheckpointMismatch,
  questCheckpointDrift,
  openingLineStepStale,
  openingLinePublicationMismatch,
  finalCheckpointMismatch,
  finalCheckpointDrift,
}

enum Revision3QuestOpeningRecipeRequiresReopenReason {
  openingCheckpoint,
  questStep,
  questCheckpoint,
  openingLineStep,
  finalCheckpoint,
}

/// Closed result of one two-checkpoint recipe attempt.
///
/// The result never claims a playable conversation. At most it proves that a
/// Quest Draft was published and that one project-only DialogLine was then
/// created and inserted into its empty transcript through the existing native
/// create-and-insert operation.
sealed class Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeOutcome({required this.openingCheckpoint});

  final ManagedRevision3CurrentProjectState openingCheckpoint;
}

/// Step one did not publish and the exact opening checkpoint is still current.
final class Revision3QuestOpeningRecipeNoChangeOutcome
    extends Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeNoChangeOutcome({
    required super.openingCheckpoint,
    required this.reason,
  });

  final Revision3QuestOpeningRecipeNoChangeReason reason;
}

/// The Quest checkpoint is valid and current, but no opening line was saved.
///
/// This is an intentional, resumable partial outcome. The Quest must not be
/// rolled back implicitly; its existing Dialog & Voice empty state is the
/// honest continuation point.
final class Revision3QuestOpeningRecipeQuestOnlyOutcome
    extends Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeQuestOnlyOutcome({
    required super.openingCheckpoint,
    required this.questStep,
    required this.reason,
  });

  final Revision3QuestOpeningRecipeQuestStep questStep;
  final Revision3QuestOpeningRecipeQuestOnlyReason reason;
}

/// Both managed checkpoints were published and exactly rebound.
final class Revision3QuestOpeningRecipeCompletedOutcome
    extends Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeCompletedOutcome({
    required super.openingCheckpoint,
    required this.questStep,
    required this.lineStep,
  });

  final Revision3QuestOpeningRecipeQuestStep questStep;
  final Revision3QuestOpeningRecipeLineStep lineStep;
}

/// The recipe lost its exact checkpoint binding and must not continue.
///
/// This does not assert that the managed project itself requires reopening. A
/// normal project refresh or manual continuation may be sufficient.
final class Revision3QuestOpeningRecipeLockedOutcome
    extends Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeLockedOutcome({
    required super.openingCheckpoint,
    required this.reason,
    this.questStep,
  });

  final Revision3QuestOpeningRecipeLockReason reason;
  final Revision3QuestOpeningRecipeQuestStep? questStep;
}

/// Exact-current verification is poisoned or publication is uncertain.
final class Revision3QuestOpeningRecipeRequiresReopenOutcome
    extends Revision3QuestOpeningRecipeOutcome {
  const Revision3QuestOpeningRecipeRequiresReopenOutcome({
    required super.openingCheckpoint,
    required this.reason,
    this.questStep,
  });

  final Revision3QuestOpeningRecipeRequiresReopenReason reason;
  final Revision3QuestOpeningRecipeQuestStep? questStep;
}

/// Pure orchestration for the bounded `Quest + opening dialog line` Draft V1.
///
/// The two existing native mutations remain separate managed revisions. This
/// class only sequences them, verifies each exact root/project/revision/head
/// checkpoint, and returns a truthful partial or locked outcome. Concurrent
/// calls on one instance share one in-flight Future so duplicate UI activation
/// cannot start a second mutation.
final class Revision3QuestOpeningRecipe {
  Future<Revision3QuestOpeningRecipeOutcome>? _inFlight;

  bool get isRunning => _inFlight != null;

  Future<Revision3QuestOpeningRecipeOutcome> run({
    required ManagedRevision3CurrentProjectState openingCheckpoint,
    required Revision3QuestOpeningRecipeCheckpointReader readCurrentCheckpoint,
    required Revision3QuestOpeningRecipeQuestStepAction createQuest,
    required Revision3QuestOpeningRecipeLineStepAction createOpeningLine,
  }) {
    final active = _inFlight;
    if (active != null) return active;

    final operation = _run(
      openingCheckpoint: openingCheckpoint,
      readCurrentCheckpoint: readCurrentCheckpoint,
      createQuest: createQuest,
      createOpeningLine: createOpeningLine,
    );
    _inFlight = operation;
    operation.then<void>(
      (_) => _clearInFlight(operation),
      onError: (_, _) => _clearInFlight(operation),
    );
    return operation;
  }

  void _clearInFlight(Future<Revision3QuestOpeningRecipeOutcome> operation) {
    if (identical(_inFlight, operation)) _inFlight = null;
  }

  Future<Revision3QuestOpeningRecipeOutcome> _run({
    required ManagedRevision3CurrentProjectState openingCheckpoint,
    required Revision3QuestOpeningRecipeCheckpointReader readCurrentCheckpoint,
    required Revision3QuestOpeningRecipeQuestStepAction createQuest,
    required Revision3QuestOpeningRecipeLineStepAction createOpeningLine,
  }) async {
    if (openingCheckpoint.requiresReopen) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        reason:
            Revision3QuestOpeningRecipeRequiresReopenReason.openingCheckpoint,
      );
    }

    final beforeQuest = await _readCheckpoint(readCurrentCheckpoint);
    if (!beforeQuest.available) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3QuestOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    final beforeQuestState = beforeQuest.checkpoint;
    if (_sameProjectRequiresReopen(beforeQuestState, openingCheckpoint)) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        reason:
            Revision3QuestOpeningRecipeRequiresReopenReason.openingCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(beforeQuestState, openingCheckpoint)) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3QuestOpeningRecipeLockReason.openingCheckpointDrift,
      );
    }

    Revision3QuestOpeningRecipeQuestStep? questStep;
    Object? questError;
    try {
      questStep = await createQuest(expectedCheckpoint: openingCheckpoint);
    } catch (error) {
      questError = error;
    }
    if (questStep == null) {
      return _classifyQuestStepWithoutPublication(
        openingCheckpoint: openingCheckpoint,
        current: await _readCheckpoint(readCurrentCheckpoint),
        error: questError,
      );
    }

    if (_sameProjectRequiresReopen(questStep.checkpoint, openingCheckpoint)) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeRequiresReopenReason.questCheckpoint,
      );
    }
    if (!_questPublicationMatches(openingCheckpoint, questStep.publication)) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.questPublicationMismatch,
      );
    }
    if (!_isNextCheckpoint(
      before: openingCheckpoint,
      after: questStep.checkpoint,
      expectedRevision: questStep.publication.projectRevision,
    )) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.questCheckpointMismatch,
      );
    }

    final reboundQuest = await _readCheckpoint(readCurrentCheckpoint);
    if (!reboundQuest.available) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    if (_sameProjectRequiresReopen(
      reboundQuest.checkpoint,
      questStep.checkpoint,
    )) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeRequiresReopenReason.questCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(reboundQuest.checkpoint, questStep.checkpoint)) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.questCheckpointDrift,
      );
    }

    final handoff = Revision3QuestOpeningRecipeHandoff._(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
    );
    Revision3QuestOpeningRecipeLineStep? lineStep;
    Object? lineError;
    try {
      lineStep = await createOpeningLine(handoff: handoff);
    } catch (error) {
      lineError = error;
    }
    if (lineStep == null) {
      return _classifyLineStepWithoutPublication(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        current: await _readCheckpoint(readCurrentCheckpoint),
        error: lineError,
      );
    }

    if (_sameProjectRequiresReopen(lineStep.checkpoint, questStep.checkpoint)) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    }
    if (!_linePublicationMatches(questStep, lineStep.publication)) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason
            .openingLinePublicationMismatch,
      );
    }
    if (!_isNextCheckpoint(
      before: questStep.checkpoint,
      after: lineStep.checkpoint,
      expectedRevision: lineStep.publication.projectRevision,
    )) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.finalCheckpointMismatch,
      );
    }

    final reboundFinal = await _readCheckpoint(readCurrentCheckpoint);
    if (!reboundFinal.available) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    if (_sameProjectRequiresReopen(
      reboundFinal.checkpoint,
      lineStep.checkpoint,
    )) {
      return Revision3QuestOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(reboundFinal.checkpoint, lineStep.checkpoint)) {
      return Revision3QuestOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        questStep: questStep,
        reason: Revision3QuestOpeningRecipeLockReason.finalCheckpointDrift,
      );
    }

    return Revision3QuestOpeningRecipeCompletedOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      lineStep: lineStep,
    );
  }
}

Revision3QuestOpeningRecipeOutcome _classifyQuestStepWithoutPublication({
  required ManagedRevision3CurrentProjectState openingCheckpoint,
  required _RecipeCheckpointRead current,
  required Object? error,
}) {
  if (_requiresReopenError(error)) {
    return Revision3QuestOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3QuestOpeningRecipeRequiresReopenReason.questStep,
    );
  }
  if (_staleError(error)) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3QuestOpeningRecipeLockReason.questStepStale,
    );
  }
  if (!current.available) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3QuestOpeningRecipeLockReason.checkpointUnavailable,
    );
  }
  if (_sameProjectRequiresReopen(current.checkpoint, openingCheckpoint)) {
    return Revision3QuestOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3QuestOpeningRecipeRequiresReopenReason.questStep,
    );
  }
  if (!_sameExactCheckpoint(current.checkpoint, openingCheckpoint)) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3QuestOpeningRecipeLockReason.openingCheckpointDrift,
    );
  }
  return Revision3QuestOpeningRecipeNoChangeOutcome(
    openingCheckpoint: openingCheckpoint,
    reason: error == null
        ? Revision3QuestOpeningRecipeNoChangeReason.cancelled
        : Revision3QuestOpeningRecipeNoChangeReason.failed,
  );
}

Revision3QuestOpeningRecipeOutcome _classifyLineStepWithoutPublication({
  required ManagedRevision3CurrentProjectState openingCheckpoint,
  required Revision3QuestOpeningRecipeQuestStep questStep,
  required _RecipeCheckpointRead current,
  required Object? error,
}) {
  if (_requiresReopenError(error)) {
    return Revision3QuestOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      reason: Revision3QuestOpeningRecipeRequiresReopenReason.openingLineStep,
    );
  }
  if (_staleError(error)) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      reason: Revision3QuestOpeningRecipeLockReason.openingLineStepStale,
    );
  }
  if (!current.available) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      reason: Revision3QuestOpeningRecipeLockReason.checkpointUnavailable,
    );
  }
  if (_sameProjectRequiresReopen(current.checkpoint, questStep.checkpoint)) {
    return Revision3QuestOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      reason: Revision3QuestOpeningRecipeRequiresReopenReason.openingLineStep,
    );
  }
  if (!_sameExactCheckpoint(current.checkpoint, questStep.checkpoint)) {
    return Revision3QuestOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      questStep: questStep,
      reason: Revision3QuestOpeningRecipeLockReason.questCheckpointDrift,
    );
  }
  return Revision3QuestOpeningRecipeQuestOnlyOutcome(
    openingCheckpoint: openingCheckpoint,
    questStep: questStep,
    reason: error == null
        ? Revision3QuestOpeningRecipeQuestOnlyReason.openingLineCancelled
        : Revision3QuestOpeningRecipeQuestOnlyReason.openingLineFailed,
  );
}

bool _questPublicationMatches(
  ManagedRevision3CurrentProjectState opening,
  Revision3QuestDraftPublication publication,
) =>
    publication.projectId == opening.projectId &&
    publication.projectRevision == opening.projectRevision + 1;

bool _linePublicationMatches(
  Revision3QuestOpeningRecipeQuestStep quest,
  Revision3QuestTranscriptPublication publication,
) =>
    publication.projectId == quest.publication.projectId &&
    publication.projectRevision == quest.checkpoint.projectRevision + 1 &&
    publication.questId == quest.publication.questId &&
    publication.moduleId == quest.publication.scriptModuleId &&
    publication.mode == AuthoringRevision3QuestTranscriptMode.createAndInsert &&
    publication.transcriptCount == 1 &&
    publication.createdLineId != null &&
    publication.createdLocalizationId != null &&
    publication.localizationAction != null;

bool _isNextCheckpoint({
  required ManagedRevision3CurrentProjectState before,
  required ManagedRevision3CurrentProjectState after,
  required int expectedRevision,
}) =>
    !after.requiresReopen &&
    after.root.path == before.root.path &&
    after.projectId == before.projectId &&
    after.projectRevision == expectedRevision &&
    expectedRevision == before.projectRevision + 1 &&
    after.head.canonicalJson != before.head.canonicalJson;

bool _sameExactCheckpoint(
  ManagedRevision3CurrentProjectState? left,
  ManagedRevision3CurrentProjectState right,
) =>
    left != null &&
    !left.requiresReopen &&
    left.root.path == right.root.path &&
    left.projectId == right.projectId &&
    left.projectRevision == right.projectRevision &&
    left.head.canonicalJson == right.head.canonicalJson;

bool _sameProjectRequiresReopen(
  ManagedRevision3CurrentProjectState? current,
  ManagedRevision3CurrentProjectState expected,
) =>
    current != null &&
    current.requiresReopen &&
    current.root.path == expected.root.path &&
    current.projectId == expected.projectId;

bool _requiresReopenError(Object? error) =>
    error is Revision3QuestDraftRequiresReopenException ||
    error is Revision3QuestTranscriptRequiresReopenException ||
    error is Revision3DialogLineEntryRequiresReopenException;

bool _staleError(Object? error) =>
    error is Revision3QuestDraftStaleCheckpointException ||
    error is Revision3QuestTranscriptStaleCheckpointException ||
    error is Revision3DialogLineEntryStaleCheckpointException;

final class _RecipeCheckpointRead {
  const _RecipeCheckpointRead._({
    required this.available,
    required this.checkpoint,
  });

  const _RecipeCheckpointRead.available(
    ManagedRevision3CurrentProjectState? checkpoint,
  ) : this._(available: true, checkpoint: checkpoint);

  const _RecipeCheckpointRead.unavailable()
    : this._(available: false, checkpoint: null);

  final bool available;
  final ManagedRevision3CurrentProjectState? checkpoint;
}

Future<_RecipeCheckpointRead> _readCheckpoint(
  Revision3QuestOpeningRecipeCheckpointReader read,
) async {
  try {
    return _RecipeCheckpointRead.available(await read());
  } catch (_) {
    return const _RecipeCheckpointRead.unavailable();
  }
}
