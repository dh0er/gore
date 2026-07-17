import 'revision3_content_index.dart';
import 'revision3_quest_journey.dart';
import 'revision3_quest_transcript_authoring.dart';
import 'revision3_quest_transitions_authoring.dart';

/// The managed project or one of its exact read receipts can no longer be
/// trusted. Callers must reopen or recover it before requesting another
/// journey projection.
final class Revision3QuestJourneyRequiresReopenException implements Exception {
  const Revision3QuestJourneyRequiresReopenException();
}

/// Read-only exact-checkpoint loader for one complete Quest journey.
///
/// The supplied services retain their existing proven read boundaries. This
/// orchestrator invokes only their `load` methods, resolves the Quest's sole
/// exact generated ScriptModule from the visible index, and composes a
/// presentation projection. It accepts no game root, performs no localization
/// text read or publication, and grants no build, deployment, runtime, or save
/// authority.
final class Revision3QuestJourneyService {
  const Revision3QuestJourneyService({
    required this.transitions,
    required this.transcript,
  });

  final Revision3QuestTransitionsAuthoringService transitions;
  final Revision3QuestTranscriptAuthoringService transcript;

  Future<Revision3QuestJourneyProjection> load({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
  }) async {
    try {
      final module = _exactQuestModule(index, quest);
      final transitionRead = transitions.load(index: index, quest: quest);
      final transcriptRead = transcript.load(
        questId: quest.id,
        expectedQuestRevision: quest.revision,
      );
      final reads = await Future.wait<Object>(<Future<Object>>[
        _settleQuestJourneyRead(transitionRead),
        _settleQuestJourneyRead(transcriptRead),
      ]);
      final failures = reads
          .whereType<_Revision3QuestJourneyReadFailure>()
          .toList(growable: false);
      if (failures.any(
        (failure) => !_isQuestJourneyStaleReadError(failure.error),
      )) {
        throw const Revision3QuestJourneyRequiresReopenException();
      }
      if (failures.isNotEmpty) {
        throw const Revision3QuestJourneyStaleCheckpointException();
      }
      final transitionCheckpoint =
          reads[0] as Revision3QuestTransitionsEditCheckpoint;
      final transcriptProjection =
          reads[1] as Revision3QuestTranscriptProjection;
      if (!identical(transitionCheckpoint.index, index) ||
          !identical(transitionCheckpoint.quest, quest)) {
        throw const Revision3QuestJourneyStaleCheckpointException();
      }
      return Revision3QuestJourneyProjection.compose(
        index: index,
        quest: quest,
        module: module,
        transitionSeed: transitionCheckpoint.seed,
        transcript: transcriptProjection,
      );
    } on Revision3QuestJourneyStaleCheckpointException {
      rethrow;
    } on Revision3QuestTransitionsStaleCheckpointException {
      throw const Revision3QuestJourneyStaleCheckpointException();
    } on Revision3QuestTranscriptStaleCheckpointException {
      throw const Revision3QuestJourneyStaleCheckpointException();
    } on FormatException {
      throw const Revision3QuestJourneyStaleCheckpointException();
    } on Revision3QuestJourneyRequiresReopenException {
      rethrow;
    } on Revision3QuestTransitionsRequiresReopenException {
      throw const Revision3QuestJourneyRequiresReopenException();
    } on Revision3QuestTranscriptRequiresReopenException {
      throw const Revision3QuestJourneyRequiresReopenException();
    } on Revision3ContentRequiresReopenException {
      throw const Revision3QuestJourneyRequiresReopenException();
    } catch (_) {
      throw const Revision3QuestJourneyRequiresReopenException();
    }
  }
}

final class _Revision3QuestJourneyReadFailure {
  const _Revision3QuestJourneyReadFailure(this.error);

  final Object error;
}

Future<Object> _settleQuestJourneyRead(Future<Object> read) async {
  try {
    return await read;
  } catch (error) {
    return _Revision3QuestJourneyReadFailure(error);
  }
}

bool _isQuestJourneyStaleReadError(Object error) =>
    error is Revision3QuestJourneyStaleCheckpointException ||
    error is Revision3QuestTransitionsStaleCheckpointException ||
    error is Revision3QuestTranscriptStaleCheckpointException ||
    error is FormatException;

Revision3ContentEntity _exactQuestModule(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  if (quest.kind != Revision3ContentEntityKind.questDraft ||
      quest.summary.questDraft == null ||
      quest.problemCount != 0 ||
      !identical(index.entityById(quest.id), quest)) {
    throw const Revision3QuestJourneyStaleCheckpointException();
  }
  final references = quest.references
      .where((reference) => reference.role == 'draft_script_module')
      .toList(growable: false);
  if (references.length != 1) {
    throw const Revision3QuestJourneyStaleCheckpointException();
  }
  final reference = references.single;
  final module = index.entityById(reference.target.entityId);
  if (reference.qualifier != null ||
      reference.resolution != Revision3ContentReferenceResolution.resolved ||
      reference.target.projectId != index.projectId ||
      reference.target.expectedKind !=
          Revision3ContentEntityKind.scriptModule ||
      module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule ||
      module.problemCount != 0) {
    throw const Revision3QuestJourneyStaleCheckpointException();
  }
  return module;
}
