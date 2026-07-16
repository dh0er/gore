import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3DialogVoiceSlotRemovalTechnicalPublisher =
    Future<Revision3DialogVoiceSlotRemovalPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
    });

/// Exact hidden intent derived from one fresh Voice catalog. A slot can be
/// removed only after the catalog proved it intact, empty, and unselected.
final class Revision3DialogVoiceSlotRemovalTechnicalPlan {
  const Revision3DialogVoiceSlotRemovalTechnicalPlan._({
    required this.lineId,
    required this.expectedLineRevision,
    required this.localizationId,
    required this.locId,
    required this.locale,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.targetResolution,
  });

  factory Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
  }) {
    final line = catalog.line(lineId);
    final slotId = line?.slotIdForLocale(locale);
    final summary = line?.slotSummaryForLocale(locale);
    if (line == null ||
        slotId == null ||
        summary == null ||
        !summary.isRemovableGeneratedSlot ||
        summary.candidateCount != 0 ||
        summary.hasSelectedTake ||
        summary.selectedTakeId != null) {
      throw const Revision3DialogVoiceSlotRemovalStaleCheckpointException();
    }
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(
      line.localizationIdentity,
    )) {
      throw const FormatException(
        'This dialog line has no safe Voice localization identity.',
      );
    }
    return Revision3DialogVoiceSlotRemovalTechnicalPlan._(
      lineId: line.lineId,
      expectedLineRevision: line.lineRevision,
      localizationId: line.localizationId,
      locId: line.localizationIdentity,
      locale: locale,
      slotId: slotId,
      expectedSlotRevision: summary.slotRevision,
      targetResolution: summary.targetResolution,
    );
  }

  final String lineId;
  final int expectedLineRevision;
  final String localizationId;
  final String locId;
  final String locale;
  final String slotId;
  final int expectedSlotRevision;
  final Revision3ContentVoiceTargetResolution targetResolution;
}

/// Result returned only after fixed-head publication and a complete reopen.
/// It grants no game, build, runtime, deployment, save, or media authority.
final class Revision3DialogVoiceSlotRemovalPublication {
  Revision3DialogVoiceSlotRemovalPublication({
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.slotId,
    required this.removedSlotRevision,
    required this.locale,
    required this.locId,
    required this.removedTargetResolution,
  }) {
    if (!_dialogVoiceSlotRemovalEntityId.hasMatch(projectId) ||
        _dialogVoiceSlotRemovalIsZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        <String>{lineId, localizationId, slotId}.length != 3 ||
        [lineId, localizationId, slotId].any(
          (id) =>
              !_dialogVoiceSlotRemovalEntityId.hasMatch(id) ||
              _dialogVoiceSlotRemovalIsZeroId(id),
        ) ||
        lineRevision < 0 ||
        lineRevision > 0x7fffffffffffffff ||
        removedSlotRevision < 0 ||
        removedSlotRevision > 0x7fffffffffffffff ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
      throw const FormatException(
        'Dialog Voice slot removal publication is invalid.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final String lineId;
  final int lineRevision;
  final String localizationId;
  final String slotId;
  final int removedSlotRevision;
  final String locale;
  final String locId;
  final Revision3ContentVoiceTargetResolution removedTargetResolution;
}

final class Revision3DialogVoiceSlotRemovalRequiresReopenException
    implements Exception {
  const Revision3DialogVoiceSlotRemovalRequiresReopenException();
}

final class Revision3DialogVoiceSlotRemovalStaleCheckpointException
    implements Exception {
  const Revision3DialogVoiceSlotRemovalStaleCheckpointException();
}

/// Fresh-index boundary for removing one exact empty dialog Voice slot.
final class Revision3DialogVoiceSlotRemovalAuthoringService {
  const Revision3DialogVoiceSlotRemovalAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3DialogVoiceSlotRemovalTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
    }
  }

  Future<Revision3DialogVoiceSlotRemovalPublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3DialogVoiceSlotRemovalStaleCheckpointException();
    }
    final plan = Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.lineRevision != plan.expectedLineRevision + 1 ||
        publication.localizationId != plan.localizationId ||
        publication.slotId != plan.slotId ||
        publication.removedSlotRevision != plan.expectedSlotRevision ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId ||
        publication.removedTargetResolution != plan.targetResolution) {
      throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
    }
    return publication;
  }
}

final _dialogVoiceSlotRemovalEntityId = RegExp(r'^[0-9a-f]{32}$');

bool _dialogVoiceSlotRemovalIsZeroId(String value) =>
    value == '00000000000000000000000000000000';
