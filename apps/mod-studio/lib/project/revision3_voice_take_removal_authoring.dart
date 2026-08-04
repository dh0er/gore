import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceTakeRemovalTechnicalPublisher =
    Future<Revision3VoiceTakeRemovalPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakeRemovalTechnicalPlan plan,
    });

/// Exact hidden detach intent derived from one fresh Voice catalog. Normal UI
/// supplies only the visible line/language/take choice.
final class Revision3VoiceTakeRemovalTechnicalPlan {
  const Revision3VoiceTakeRemovalTechnicalPlan._({
    required this.lineId,
    required this.localizationId,
    required this.locId,
    required this.locale,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.takeId,
    required this.expectedTakeRevision,
    required this.expectedSelectedTakeId,
    required this.expectedRemainingCandidateCount,
    required this.expectedTakeEntityRemoved,
  });

  factory Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
    required String takeId,
  }) {
    final line = catalog.line(lineId);
    if (line == null) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    final slotId = line.slotIdForLocale(locale);
    final summary = line.slotSummaryForLocale(locale);
    final take = summary?.candidate(takeId);
    if (slotId == null || summary == null || take == null) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(
      line.localizationIdentity,
    )) {
      throw const FormatException(
        'This dialog line has no safe Voice localization identity.',
      );
    }
    if (summary.candidateCount < 1) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    final candidateSlotUseCount = catalog.candidateSlotUseCount(take.id);
    if (candidateSlotUseCount < 1 ||
        !catalog.candidateIsUsedBySlot(take.id, slotId)) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    return Revision3VoiceTakeRemovalTechnicalPlan._(
      lineId: line.lineId,
      localizationId: line.localizationId,
      locId: line.localizationIdentity,
      locale: locale,
      slotId: slotId,
      expectedSlotRevision: summary.slotRevision,
      takeId: take.id,
      expectedTakeRevision: take.revision,
      expectedSelectedTakeId: summary.selectedTakeId,
      expectedRemainingCandidateCount: summary.candidateCount - 1,
      expectedTakeEntityRemoved: candidateSlotUseCount == 1,
    );
  }

  final String lineId;
  final String localizationId;
  final String locId;
  final String locale;
  final String slotId;
  final int expectedSlotRevision;
  final String takeId;
  final int expectedTakeRevision;
  final String? expectedSelectedTakeId;
  final int expectedRemainingCandidateCount;
  final bool expectedTakeEntityRemoved;

  bool get expectsSelectionCleared => expectedSelectedTakeId == takeId;
}

/// One detached take returned only after the managed candidate was published
/// by fixed-head CAS and fully reopened. Audio CAS metadata remains preserved;
/// this grants no build, runtime, deployment, game, save, or artifact authority.
final class Revision3VoiceTakeRemovalPublication {
  Revision3VoiceTakeRemovalPublication({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.takeId,
    required this.takeRevision,
    required this.previousSelectedTakeId,
    required this.selectionCleared,
    required this.takeEntityRemoved,
    required this.remainingCandidateCount,
  }) {
    if (!_voiceTakeRemovalEntityId.hasMatch(projectId) ||
        _voiceTakeRemovalIsZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        <String>{lineId, localizationId, slotId, takeId}.length != 4 ||
        [lineId, localizationId, slotId, takeId].any(
          (id) =>
              !_voiceTakeRemovalEntityId.hasMatch(id) ||
              _voiceTakeRemovalIsZeroId(id),
        ) ||
        slotRevision < 0 ||
        slotRevision > 0x7fffffffffffffff ||
        takeRevision < 0 ||
        takeRevision > 0x7fffffffffffffff ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        (previousSelectedTakeId != null &&
            (!_voiceTakeRemovalEntityId.hasMatch(previousSelectedTakeId!) ||
                _voiceTakeRemovalIsZeroId(previousSelectedTakeId!))) ||
        selectionCleared != (previousSelectedTakeId == takeId) ||
        remainingCandidateCount < 0 ||
        remainingCandidateCount >= 1024) {
      throw const FormatException('Voice take removal publication is invalid.');
    }
  }

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String takeId;
  final int takeRevision;
  final String? previousSelectedTakeId;
  final bool selectionCleared;
  final bool takeEntityRemoved;
  final int remainingCandidateCount;
}

final class Revision3VoiceTakeRemovalRequiresReopenException
    implements Exception {
  const Revision3VoiceTakeRemovalRequiresReopenException();
}

final class Revision3VoiceTakeRemovalStaleCheckpointException
    implements Exception {
  const Revision3VoiceTakeRemovalStaleCheckpointException();
}

/// Fresh-index boundary for detaching one exact listed Voice take. It never
/// retries automatically: any stale graph is returned to the UI for refresh.
final class Revision3VoiceTakeRemovalAuthoringService {
  const Revision3VoiceTakeRemovalAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTakeRemovalTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3VoiceTakeRemovalRequiresReopenException();
    }
  }

  Future<Revision3VoiceTakeRemovalPublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
    required String takeId,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    final plan = Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
      takeId: takeId,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.localizationId != plan.localizationId ||
        publication.slotId != plan.slotId ||
        publication.slotRevision != plan.expectedSlotRevision + 1 ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId ||
        publication.takeId != plan.takeId ||
        publication.takeRevision != plan.expectedTakeRevision ||
        publication.previousSelectedTakeId != plan.expectedSelectedTakeId ||
        publication.selectionCleared != plan.expectsSelectionCleared ||
        publication.takeEntityRemoved != plan.expectedTakeEntityRemoved ||
        publication.remainingCandidateCount !=
            plan.expectedRemainingCandidateCount) {
      throw const Revision3VoiceTakeRemovalRequiresReopenException();
    }
    return publication;
  }
}

final _voiceTakeRemovalEntityId = RegExp(r'^[0-9a-f]{32}$');

bool _voiceTakeRemovalIsZeroId(String value) =>
    value == '00000000000000000000000000000000';
