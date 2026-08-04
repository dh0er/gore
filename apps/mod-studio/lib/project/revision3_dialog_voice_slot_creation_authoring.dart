import 'dart:convert';

import 'package:crypto/crypto.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3DialogVoiceSlotCreationTechnicalPublisher =
    Future<Revision3DialogVoiceSlotCreationPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3DialogVoiceSlotCreationTechnicalPlan plan,
    });

/// Exact hidden intent derived from one fresh Voice catalog.
///
/// The planned slot identity is deterministic and collision-probed. It is not
/// editable or rendered in normal UI.
final class Revision3DialogVoiceSlotCreationTechnicalPlan {
  const Revision3DialogVoiceSlotCreationTechnicalPlan._({
    required this.lineId,
    required this.expectedLineRevision,
    required this.localizationId,
    required this.expectedLocalizationRevision,
    required this.locId,
    required this.locale,
    required this.slotId,
  });

  factory Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
  }) {
    final line = catalog.line(lineId);
    if (line == null ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        line.slotIdForLocale(locale) != null ||
        !line.isLocaleAuthorable(locale)) {
      throw const Revision3DialogVoiceSlotCreationStaleCheckpointException();
    }
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(
      line.localizationIdentity,
    )) {
      throw const FormatException(
        'This dialog line has no safe Voice localization identity.',
      );
    }
    final used = <String>{...catalog.entityIds};
    final seed = jsonEncode(<String, Object?>{
      'project_id': catalog.projectId,
      'line_id': line.lineId,
      'locale': locale,
    });
    final slotId = _deriveUnusedDialogVoiceSlotId(seed, used);
    return Revision3DialogVoiceSlotCreationTechnicalPlan._(
      lineId: line.lineId,
      expectedLineRevision: line.lineRevision,
      localizationId: line.localizationId,
      expectedLocalizationRevision: line.localizationRevision,
      locId: line.localizationIdentity,
      locale: locale,
      slotId: slotId,
    );
  }

  final String lineId;
  final int expectedLineRevision;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String locId;
  final String locale;
  final String slotId;
}

/// Result returned only after fixed-head publication and a complete reopen.
/// It grants no audio, game, build, runtime, deployment, save, or target
/// authority.
final class Revision3DialogVoiceSlotCreationPublication {
  Revision3DialogVoiceSlotCreationPublication({
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.targetResolution,
  }) {
    if (!_dialogVoiceSlotCreationEntityId.hasMatch(projectId) ||
        _dialogVoiceSlotCreationIsZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        <String>{lineId, localizationId, slotId}.length != 3 ||
        [lineId, localizationId, slotId].any(
          (id) =>
              !_dialogVoiceSlotCreationEntityId.hasMatch(id) ||
              _dialogVoiceSlotCreationIsZeroId(id),
        ) ||
        lineRevision < 0 ||
        lineRevision > 0x7fffffffffffffff ||
        localizationRevision < 0 ||
        localizationRevision > 0x7fffffffffffffff ||
        slotRevision != 0 ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        targetResolution != Revision3ContentVoiceTargetResolution.unresolved) {
      throw const FormatException(
        'Dialog Voice slot creation publication is invalid.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final String lineId;
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final Revision3ContentVoiceTargetResolution targetResolution;
}

final class Revision3DialogVoiceSlotCreationRequiresReopenException
    implements Exception {
  const Revision3DialogVoiceSlotCreationRequiresReopenException();
}

final class Revision3DialogVoiceSlotCreationStaleCheckpointException
    implements Exception {
  const Revision3DialogVoiceSlotCreationStaleCheckpointException();
}

/// Fresh-index boundary for planning one exact dialog Voice recording.
final class Revision3DialogVoiceSlotCreationAuthoringService {
  const Revision3DialogVoiceSlotCreationAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3DialogVoiceSlotCreationTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3DialogVoiceSlotCreationRequiresReopenException();
    }
  }

  Future<Revision3DialogVoiceSlotCreationPublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3DialogVoiceSlotCreationStaleCheckpointException();
    }
    final plan = Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
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
        publication.localizationRevision != plan.expectedLocalizationRevision ||
        publication.slotId != plan.slotId ||
        publication.slotRevision != 0 ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId ||
        publication.targetResolution !=
            Revision3ContentVoiceTargetResolution.unresolved) {
      throw const Revision3DialogVoiceSlotCreationRequiresReopenException();
    }
    return publication;
  }
}

String _deriveUnusedDialogVoiceSlotId(String seed, Set<String> used) {
  for (var counter = 0; counter <= used.length + 1; counter++) {
    final digest = sha256
        .convert(
          utf8.encode(
            'gore-mod-studio.r3-voice-slot-id-v1\u0000$seed\u0000$counter',
          ),
        )
        .toString();
    final candidate = digest.substring(0, 32);
    if (!_dialogVoiceSlotCreationIsZeroId(candidate) &&
        !used.contains(candidate)) {
      return candidate;
    }
  }
  throw StateError(
    'A collision-free Voice slot identity could not be derived.',
  );
}

final _dialogVoiceSlotCreationEntityId = RegExp(r'^[0-9a-f]{32}$');

bool _dialogVoiceSlotCreationIsZeroId(String value) =>
    value == '00000000000000000000000000000000';
