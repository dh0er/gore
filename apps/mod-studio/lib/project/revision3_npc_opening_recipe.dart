import '../core/mod_ffi.dart';
import 'current_project_controller.dart';
import 'revision3_dialog_line_authoring.dart';
import 'revision3_npc_authoring.dart';
import 'revision3_npc_greeting_authoring.dart';

typedef Revision3NpcOpeningRecipeCheckpointReader =
    Future<ManagedRevision3CurrentProjectState?> Function();

typedef Revision3NpcOpeningRecipeNpcStepAction =
    Future<Revision3NpcOpeningRecipeNpcStep?> Function({
      required ManagedRevision3CurrentProjectState expectedCheckpoint,
    });

typedef Revision3NpcOpeningRecipeGreetingStepAction =
    Future<Revision3NpcOpeningRecipeGreetingStep?> Function({
      required Revision3NpcOpeningRecipeHandoff handoff,
    });

/// The exact NPC publication and fully reopened managed checkpoint observed
/// immediately after the Character Draft was published.
final class Revision3NpcOpeningRecipeNpcStep {
  const Revision3NpcOpeningRecipeNpcStep({
    required this.publication,
    required this.checkpoint,
  });

  final Revision3NpcDraftPublication publication;
  final ManagedRevision3CurrentProjectState checkpoint;
}

/// The exact create-and-insert Greeting publication and its fully reopened
/// managed checkpoint.
final class Revision3NpcOpeningRecipeGreetingStep {
  const Revision3NpcOpeningRecipeGreetingStep({
    required this.publication,
    required this.checkpoint,
  });

  final Revision3NpcGreetingPublication publication;
  final ManagedRevision3CurrentProjectState checkpoint;
}

/// Exact handoff from the published NPC-only checkpoint to Greeting authoring.
///
/// Hosts must bind every Greeting read and publication to [npcCheckpoint],
/// including its root and canonical WorkingHead bytes.
final class Revision3NpcOpeningRecipeHandoff {
  const Revision3NpcOpeningRecipeHandoff._({
    required this.openingCheckpoint,
    required this.npcStep,
  });

  final ManagedRevision3CurrentProjectState openingCheckpoint;
  final Revision3NpcOpeningRecipeNpcStep npcStep;

  Revision3NpcDraftPublication get npcPublication => npcStep.publication;

  ManagedRevision3CurrentProjectState get npcCheckpoint => npcStep.checkpoint;
}

enum Revision3NpcOpeningRecipeNoChangeReason { cancelled, failed }

enum Revision3NpcOpeningRecipeNpcOnlyReason {
  greetingCancelled,
  greetingFailed,
}

enum Revision3NpcOpeningRecipeLockReason {
  checkpointUnavailable,
  openingCheckpointDrift,
  npcStepStale,
  npcPublicationMismatch,
  npcCheckpointMismatch,
  npcCheckpointDrift,
  greetingStepStale,
  greetingPublicationMismatch,
  finalCheckpointMismatch,
  finalCheckpointDrift,
}

enum Revision3NpcOpeningRecipeRequiresReopenReason {
  openingCheckpoint,
  npcStep,
  npcCheckpoint,
  greetingStep,
  finalCheckpoint,
}

/// Closed result of one two-checkpoint NPC opening recipe attempt.
///
/// The result never claims a playable conversation or runtime publication. At
/// most it proves that a Character Draft was published and one project-only
/// DialogLine was then created and inserted into its empty Greeting list.
sealed class Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeOutcome({required this.openingCheckpoint});

  final ManagedRevision3CurrentProjectState openingCheckpoint;
}

/// Step one did not publish and the exact opening checkpoint remains current.
final class Revision3NpcOpeningRecipeNoChangeOutcome
    extends Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeNoChangeOutcome({
    required super.openingCheckpoint,
    required this.reason,
  });

  final Revision3NpcOpeningRecipeNoChangeReason reason;
}

/// The NPC checkpoint is valid and current, but no Greeting line was saved.
///
/// This is an intentional resumable partial outcome. The NPC must not be
/// rolled back implicitly; its empty Greeting list is the honest continuation
/// point.
final class Revision3NpcOpeningRecipeNpcOnlyOutcome
    extends Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeNpcOnlyOutcome({
    required super.openingCheckpoint,
    required this.npcStep,
    required this.reason,
  });

  final Revision3NpcOpeningRecipeNpcStep npcStep;
  final Revision3NpcOpeningRecipeNpcOnlyReason reason;
}

/// Both managed checkpoints were published and exactly rebound.
final class Revision3NpcOpeningRecipeCompletedOutcome
    extends Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeCompletedOutcome({
    required super.openingCheckpoint,
    required this.npcStep,
    required this.greetingStep,
  });

  final Revision3NpcOpeningRecipeNpcStep npcStep;
  final Revision3NpcOpeningRecipeGreetingStep greetingStep;
}

/// The recipe lost exact checkpoint binding and must not continue.
final class Revision3NpcOpeningRecipeLockedOutcome
    extends Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeLockedOutcome({
    required super.openingCheckpoint,
    required this.reason,
    this.npcStep,
  });

  final Revision3NpcOpeningRecipeLockReason reason;
  final Revision3NpcOpeningRecipeNpcStep? npcStep;
}

/// Exact-current verification is poisoned or publication is uncertain.
final class Revision3NpcOpeningRecipeRequiresReopenOutcome
    extends Revision3NpcOpeningRecipeOutcome {
  const Revision3NpcOpeningRecipeRequiresReopenOutcome({
    required super.openingCheckpoint,
    required this.reason,
    this.npcStep,
  });

  final Revision3NpcOpeningRecipeRequiresReopenReason reason;
  final Revision3NpcOpeningRecipeNpcStep? npcStep;
}

/// Pure orchestration for the bounded `Character Draft + first Greeting` flow.
///
/// The existing native mutations remain separate managed revisions. This class
/// only sequences them, verifies every exact root/project/revision/head
/// checkpoint and returns truthful partial or locked outcomes. Concurrent calls
/// on one instance share one in-flight Future.
final class Revision3NpcOpeningRecipe {
  Future<Revision3NpcOpeningRecipeOutcome>? _inFlight;

  bool get isRunning => _inFlight != null;

  Future<Revision3NpcOpeningRecipeOutcome> run({
    required ManagedRevision3CurrentProjectState openingCheckpoint,
    required Revision3NpcOpeningRecipeCheckpointReader readCurrentCheckpoint,
    required Revision3NpcOpeningRecipeNpcStepAction createNpc,
    required Revision3NpcOpeningRecipeGreetingStepAction createGreeting,
  }) {
    final active = _inFlight;
    if (active != null) return active;

    final operation = _run(
      openingCheckpoint: openingCheckpoint,
      readCurrentCheckpoint: readCurrentCheckpoint,
      createNpc: createNpc,
      createGreeting: createGreeting,
    );
    _inFlight = operation;
    operation.then<void>(
      (_) => _clearInFlight(operation),
      onError: (_, _) => _clearInFlight(operation),
    );
    return operation;
  }

  void _clearInFlight(Future<Revision3NpcOpeningRecipeOutcome> operation) {
    if (identical(_inFlight, operation)) _inFlight = null;
  }

  Future<Revision3NpcOpeningRecipeOutcome> _run({
    required ManagedRevision3CurrentProjectState openingCheckpoint,
    required Revision3NpcOpeningRecipeCheckpointReader readCurrentCheckpoint,
    required Revision3NpcOpeningRecipeNpcStepAction createNpc,
    required Revision3NpcOpeningRecipeGreetingStepAction createGreeting,
  }) async {
    if (openingCheckpoint.requiresReopen) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.openingCheckpoint,
      );
    }

    final beforeNpc = await _readCheckpoint(readCurrentCheckpoint);
    if (!beforeNpc.available) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    final beforeNpcState = beforeNpc.checkpoint;
    if (_sameProjectRequiresReopen(beforeNpcState, openingCheckpoint)) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.openingCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(beforeNpcState, openingCheckpoint)) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        reason: Revision3NpcOpeningRecipeLockReason.openingCheckpointDrift,
      );
    }

    Revision3NpcOpeningRecipeNpcStep? npcStep;
    Object? npcError;
    try {
      npcStep = await createNpc(expectedCheckpoint: openingCheckpoint);
    } catch (error) {
      npcError = error;
    }
    if (npcStep == null) {
      return _classifyNpcStepWithoutPublication(
        openingCheckpoint: openingCheckpoint,
        current: await _readCheckpoint(readCurrentCheckpoint),
        error: npcError,
      );
    }

    if (_sameProjectRequiresReopen(npcStep.checkpoint, openingCheckpoint)) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.npcCheckpoint,
      );
    }
    if (!_npcPublicationMatches(openingCheckpoint, npcStep)) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.npcPublicationMismatch,
      );
    }
    if (!_isNextCheckpoint(
      before: openingCheckpoint,
      after: npcStep.checkpoint,
      expectedRevision: npcStep.publication.projectRevision,
    )) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.npcCheckpointMismatch,
      );
    }

    final reboundNpc = await _readCheckpoint(readCurrentCheckpoint);
    if (!reboundNpc.available) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    if (_sameProjectRequiresReopen(reboundNpc.checkpoint, npcStep.checkpoint)) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.npcCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(reboundNpc.checkpoint, npcStep.checkpoint)) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.npcCheckpointDrift,
      );
    }

    final handoff = Revision3NpcOpeningRecipeHandoff._(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
    );
    Revision3NpcOpeningRecipeGreetingStep? greetingStep;
    Object? greetingError;
    try {
      greetingStep = await createGreeting(handoff: handoff);
    } catch (error) {
      greetingError = error;
    }
    if (greetingStep == null) {
      return _classifyGreetingStepWithoutPublication(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        current: await _readCheckpoint(readCurrentCheckpoint),
        error: greetingError,
      );
    }

    if (_sameProjectRequiresReopen(
      greetingStep.checkpoint,
      npcStep.checkpoint,
    )) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    }
    if (!_greetingPublicationMatches(npcStep, greetingStep.publication)) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.greetingPublicationMismatch,
      );
    }
    if (!_isNextCheckpoint(
      before: npcStep.checkpoint,
      after: greetingStep.checkpoint,
      expectedRevision: greetingStep.publication.projectRevision,
    )) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.finalCheckpointMismatch,
      );
    }

    final reboundFinal = await _readCheckpoint(readCurrentCheckpoint);
    if (!reboundFinal.available) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
      );
    }
    if (_sameProjectRequiresReopen(
      reboundFinal.checkpoint,
      greetingStep.checkpoint,
    )) {
      return Revision3NpcOpeningRecipeRequiresReopenOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    }
    if (!_sameExactCheckpoint(
      reboundFinal.checkpoint,
      greetingStep.checkpoint,
    )) {
      return Revision3NpcOpeningRecipeLockedOutcome(
        openingCheckpoint: openingCheckpoint,
        npcStep: npcStep,
        reason: Revision3NpcOpeningRecipeLockReason.finalCheckpointDrift,
      );
    }

    return Revision3NpcOpeningRecipeCompletedOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      greetingStep: greetingStep,
    );
  }
}

Revision3NpcOpeningRecipeOutcome _classifyNpcStepWithoutPublication({
  required ManagedRevision3CurrentProjectState openingCheckpoint,
  required _RecipeCheckpointRead current,
  required Object? error,
}) {
  if (_requiresReopenError(error)) {
    return Revision3NpcOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3NpcOpeningRecipeRequiresReopenReason.npcStep,
    );
  }
  if (_staleError(error)) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3NpcOpeningRecipeLockReason.npcStepStale,
    );
  }
  if (!current.available) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
    );
  }
  if (_sameProjectRequiresReopen(current.checkpoint, openingCheckpoint)) {
    return Revision3NpcOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3NpcOpeningRecipeRequiresReopenReason.npcStep,
    );
  }
  if (!_sameExactCheckpoint(current.checkpoint, openingCheckpoint)) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      reason: Revision3NpcOpeningRecipeLockReason.openingCheckpointDrift,
    );
  }
  return Revision3NpcOpeningRecipeNoChangeOutcome(
    openingCheckpoint: openingCheckpoint,
    reason: error == null
        ? Revision3NpcOpeningRecipeNoChangeReason.cancelled
        : Revision3NpcOpeningRecipeNoChangeReason.failed,
  );
}

Revision3NpcOpeningRecipeOutcome _classifyGreetingStepWithoutPublication({
  required ManagedRevision3CurrentProjectState openingCheckpoint,
  required Revision3NpcOpeningRecipeNpcStep npcStep,
  required _RecipeCheckpointRead current,
  required Object? error,
}) {
  if (_requiresReopenError(error)) {
    return Revision3NpcOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      reason: Revision3NpcOpeningRecipeRequiresReopenReason.greetingStep,
    );
  }
  if (_staleError(error)) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      reason: Revision3NpcOpeningRecipeLockReason.greetingStepStale,
    );
  }
  if (!current.available) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      reason: Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
    );
  }
  if (_sameProjectRequiresReopen(current.checkpoint, npcStep.checkpoint)) {
    return Revision3NpcOpeningRecipeRequiresReopenOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      reason: Revision3NpcOpeningRecipeRequiresReopenReason.greetingStep,
    );
  }
  if (!_sameExactCheckpoint(current.checkpoint, npcStep.checkpoint)) {
    return Revision3NpcOpeningRecipeLockedOutcome(
      openingCheckpoint: openingCheckpoint,
      npcStep: npcStep,
      reason: Revision3NpcOpeningRecipeLockReason.npcCheckpointDrift,
    );
  }
  return Revision3NpcOpeningRecipeNpcOnlyOutcome(
    openingCheckpoint: openingCheckpoint,
    npcStep: npcStep,
    reason: error == null
        ? Revision3NpcOpeningRecipeNpcOnlyReason.greetingCancelled
        : Revision3NpcOpeningRecipeNpcOnlyReason.greetingFailed,
  );
}

bool _npcPublicationMatches(
  ManagedRevision3CurrentProjectState opening,
  Revision3NpcOpeningRecipeNpcStep step,
) =>
    step.publication.projectId == opening.projectId &&
    step.publication.projectRevision == opening.projectRevision + 1 &&
    step.publication.head.canonicalJson == step.checkpoint.head.canonicalJson &&
    step.publication.head.canonicalJson != opening.head.canonicalJson;

bool _greetingPublicationMatches(
  Revision3NpcOpeningRecipeNpcStep npc,
  Revision3NpcGreetingPublication publication,
) =>
    publication.projectId == npc.publication.projectId &&
    publication.projectRevision == npc.checkpoint.projectRevision + 1 &&
    publication.npcId == npc.publication.npcId &&
    publication.moduleId == npc.publication.scriptModuleId &&
    publication.mode == AuthoringRevision3NpcGreetingMode.createAndInsert &&
    publication.greetingCount == 1 &&
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
    error is Revision3NpcDraftRequiresReopenException ||
    error is Revision3NpcGreetingRequiresReopenException ||
    error is Revision3DialogLineEntryRequiresReopenException;

bool _staleError(Object? error) =>
    error is Revision3NpcDraftStaleCheckpointException ||
    error is Revision3NpcGreetingStaleCheckpointException ||
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
  Revision3NpcOpeningRecipeCheckpointReader read,
) async {
  try {
    return _RecipeCheckpointRead.available(await read());
  } catch (_) {
    return const _RecipeCheckpointRead.unavailable();
  }
}
