import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceTakeStatusTechnicalPublisher =
    Future<Revision3VoiceTakeStatusPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakeStatusTechnicalPlan plan,
    });

/// Exact hidden status-edit intent derived from one fresh Voice catalog.
///
/// Normal UI supplies only a visible take and desired workflow status. Project,
/// graph, revision, and localization identities never become editable fields.
final class Revision3VoiceTakeStatusTechnicalPlan {
  const Revision3VoiceTakeStatusTechnicalPlan._({
    required this.lineId,
    required this.localizationId,
    required this.locId,
    required this.locale,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.takeId,
    required this.expectedTakeRevision,
    required this.expectedStatus,
    required this.desiredStatus,
  });

  factory Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
    required String takeId,
    required AuthoringRevision3VoiceTakeStatus desiredStatus,
  }) {
    final line = catalog.line(lineId);
    if (line == null) {
      throw const Revision3VoiceTakeStatusStaleCheckpointException();
    }
    final slotId = line.slotIdForLocale(locale);
    final summary = line.slotSummaryForLocale(locale);
    final take = summary?.candidate(takeId);
    if (slotId == null || summary == null || take == null) {
      throw const Revision3VoiceTakeStatusStaleCheckpointException();
    }
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(
      line.localizationIdentity,
    )) {
      throw const FormatException(
        'This dialog line has no safe Voice localization identity.',
      );
    }
    final expectedStatus = _authoringStatus(take.status);
    if (expectedStatus == desiredStatus) {
      throw const FormatException('Choose a different take status.');
    }
    if (summary.selectedTakeId == takeId &&
        desiredStatus != AuthoringRevision3VoiceTakeStatus.approved) {
      throw const Revision3VoiceTakeStatusSelectedTakeException();
    }
    return Revision3VoiceTakeStatusTechnicalPlan._(
      lineId: line.lineId,
      localizationId: line.localizationId,
      locId: line.localizationIdentity,
      locale: locale,
      slotId: slotId,
      expectedSlotRevision: summary.slotRevision,
      takeId: take.id,
      expectedTakeRevision: take.revision,
      expectedStatus: expectedStatus,
      desiredStatus: desiredStatus,
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
  final AuthoringRevision3VoiceTakeStatus expectedStatus;
  final AuthoringRevision3VoiceTakeStatus desiredStatus;
}

/// One status edit returned only after the managed head was published and
/// fully reopened. It grants no audio-quality, build, deployment, or runtime
/// authority.
final class Revision3VoiceTakeStatusPublication {
  Revision3VoiceTakeStatusPublication({
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
    required this.previousStatus,
    required this.status,
  }) {
    if (!_entityId.hasMatch(projectId) ||
        _isZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        <String>{lineId, localizationId, slotId, takeId}.length != 4 ||
        [
          lineId,
          localizationId,
          slotId,
          takeId,
        ].any((id) => !_entityId.hasMatch(id) || _isZeroId(id)) ||
        slotRevision < 0 ||
        slotRevision > 0x7fffffffffffffff ||
        takeRevision < 0 ||
        takeRevision > 0x7fffffffffffffff ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        previousStatus == status) {
      throw const FormatException('Voice take status publication is invalid.');
    }
  }

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
  final AuthoringRevision3VoiceTakeStatus previousStatus;
  final AuthoringRevision3VoiceTakeStatus status;
}

final class Revision3VoiceTakeStatusRequiresReopenException
    implements Exception {
  const Revision3VoiceTakeStatusRequiresReopenException();
}

final class Revision3VoiceTakeStatusStaleCheckpointException
    implements Exception {
  const Revision3VoiceTakeStatusStaleCheckpointException();
}

final class Revision3VoiceTakeStatusSelectedTakeException implements Exception {
  const Revision3VoiceTakeStatusSelectedTakeException();
}

/// Fresh-index boundary for changing one retained take's author-managed status.
final class Revision3VoiceTakeStatusAuthoringService {
  const Revision3VoiceTakeStatusAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTakeStatusTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3VoiceTakeStatusRequiresReopenException();
    }
  }

  Future<Revision3VoiceTakeStatusPublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
    required String takeId,
    required AuthoringRevision3VoiceTakeStatus desiredStatus,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakeStatusStaleCheckpointException();
    }
    final plan = Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
      takeId: takeId,
      desiredStatus: desiredStatus,
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
        publication.slotRevision != plan.expectedSlotRevision ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId ||
        publication.takeId != plan.takeId ||
        publication.takeRevision != plan.expectedTakeRevision + 1 ||
        publication.previousStatus != plan.expectedStatus ||
        publication.status != plan.desiredStatus) {
      throw const Revision3VoiceTakeStatusRequiresReopenException();
    }
    return publication;
  }
}

AuthoringRevision3VoiceTakeStatus _authoringStatus(
  Revision3ContentVoiceTakeStatus status,
) => AuthoringRevision3VoiceTakeStatus.values.byName(status.name);

final _entityId = RegExp(r'^[0-9a-f]{32}$');

bool _isZeroId(String value) => value == '00000000000000000000000000000000';
