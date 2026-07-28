import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceTakeSelectionTechnicalPublisher =
    Future<Revision3VoiceTakeSelectionPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakeSelectionTechnicalPlan plan,
    });

/// Exact hidden transaction plan derived from one visible line/locale and a
/// freshly loaded Voice catalog. Normal UI never asks an author to type IDs.
final class Revision3VoiceTakeSelectionTechnicalPlan {
  const Revision3VoiceTakeSelectionTechnicalPlan._({
    required this.lineId,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.locale,
    required this.locId,
    required this.expectedSelectedTakeId,
    required this.selectedTakeId,
  });

  factory Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
    required String? selectedTakeId,
  }) {
    final line = catalog.line(lineId);
    if (line == null) {
      throw const Revision3VoiceTakeSelectionStaleCheckpointException();
    }
    final slotId = line.slotIdForLocale(locale);
    final summary = line.slotSummaryForLocale(locale);
    if (slotId == null || summary == null) {
      throw const FormatException(
        'Choose an intact existing Voice slot from the current project.',
      );
    }
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(
      line.localizationIdentity,
    )) {
      throw const FormatException(
        'This dialog line has no safe Voice localization identity.',
      );
    }
    if (selectedTakeId == summary.selectedTakeId) {
      throw const FormatException('Choose a different Voice take.');
    }
    if (selectedTakeId != null) {
      final selected = summary.candidate(selectedTakeId);
      if (selected == null) {
        throw const Revision3VoiceTakeSelectionStaleCheckpointException();
      }
      if (!selected.isApproved) {
        throw const FormatException(
          'Only an Approved take can become the selected take.',
        );
      }
    }
    return Revision3VoiceTakeSelectionTechnicalPlan._(
      lineId: line.lineId,
      slotId: slotId,
      expectedSlotRevision: summary.slotRevision,
      locale: locale,
      locId: line.localizationIdentity,
      expectedSelectedTakeId: summary.selectedTakeId,
      selectedTakeId: selectedTakeId,
    );
  }

  final String lineId;
  final String slotId;
  final int expectedSlotRevision;
  final String locale;
  final String locId;
  final String? expectedSelectedTakeId;
  final String? selectedTakeId;
}

final class Revision3VoiceTakeSelectionPublication {
  Revision3VoiceTakeSelectionPublication({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.previousSelectedTakeId,
    required this.selectedTakeId,
  }) {
    if (!_voiceSelectionEntityId.hasMatch(projectId) ||
        _voiceSelectionIsZeroId(projectId) ||
        projectRevision < 0 ||
        slotRevision < 0 ||
        [lineId, slotId].any(
          (id) =>
              !_voiceSelectionEntityId.hasMatch(id) ||
              _voiceSelectionIsZeroId(id),
        ) ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        previousSelectedTakeId == selectedTakeId ||
        [previousSelectedTakeId, selectedTakeId].whereType<String>().any(
          (id) =>
              !_voiceSelectionEntityId.hasMatch(id) ||
              _voiceSelectionIsZeroId(id),
        )) {
      throw const FormatException(
        'Voice take selection publication is invalid.',
      );
    }
  }

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String? previousSelectedTakeId;
  final String? selectedTakeId;

  bool get cleared => selectedTakeId == null;
}

final class Revision3VoiceTakeSelectionRequiresReopenException
    implements Exception {
  const Revision3VoiceTakeSelectionRequiresReopenException();
}

final class Revision3VoiceTakeSelectionStaleCheckpointException
    implements Exception {
  const Revision3VoiceTakeSelectionStaleCheckpointException();
}

/// Fresh-index boundary for selecting or clearing one existing Voice take.
final class Revision3VoiceTakeSelectionAuthoringService {
  const Revision3VoiceTakeSelectionAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTakeSelectionTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3VoiceTakeSelectionRequiresReopenException();
    }
  }

  Future<Revision3VoiceTakeSelectionPublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
    required String? selectedTakeId,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakeSelectionStaleCheckpointException();
    }
    final plan = Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
      selectedTakeId: selectedTakeId,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.slotId != plan.slotId ||
        publication.slotRevision != plan.expectedSlotRevision + 1 ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId ||
        publication.previousSelectedTakeId != plan.expectedSelectedTakeId ||
        publication.selectedTakeId != plan.selectedTakeId) {
      throw const Revision3VoiceTakeSelectionRequiresReopenException();
    }
    return publication;
  }
}

final _voiceSelectionEntityId = RegExp(r'^[0-9a-f]{32}$');

bool _voiceSelectionIsZeroId(String value) =>
    value == '00000000000000000000000000000000';
