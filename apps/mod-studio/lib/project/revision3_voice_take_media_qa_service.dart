import '../core/mod_ffi.dart';
import 'managed_project_session.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_preview_authoring.dart';

typedef Revision3VoiceTakeMediaQaTechnicalInspector =
    Future<AuthoringRevision3VoiceTakeMediaQaResult> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakePreviewTechnicalPlan plan,
    });

/// The visible Voice catalog or one of the selected graph leaves changed.
/// Reloading the catalog is sufficient; the managed session remains usable.
final class Revision3VoiceTakeMediaQaStaleCheckpointException
    implements Exception {
  const Revision3VoiceTakeMediaQaStaleCheckpointException();
}

/// Exact Store, response, or session authority became uncertain. Callers must
/// stop offering further QA work until the project has been verified/reopened.
final class Revision3VoiceTakeMediaQaRequiresReopenException
    implements Exception {
  const Revision3VoiceTakeMediaQaRequiresReopenException({this.cause});

  final Object? cause;
}

/// Fresh-index boundary for pathless native media QA. It deliberately reuses
/// the exact Preview graph/asset binding instead of accepting UI-authored IDs.
final class Revision3VoiceTakeMediaQaAuthoringService {
  const Revision3VoiceTakeMediaQaAuthoringService({
    required this.loadContentIndex,
    required this.inspectTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTakeMediaQaTechnicalInspector inspectTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException catch (error) {
      throw Revision3VoiceTakeMediaQaRequiresReopenException(cause: error);
    }
  }

  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspect({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
    required String takeId,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakeMediaQaStaleCheckpointException();
    }

    final Revision3VoiceTakePreviewTechnicalPlan plan;
    try {
      plan = Revision3VoiceTakePreviewTechnicalPlan.forCheckpoint(
        catalog: fresh,
        lineId: lineId,
        locale: locale,
        takeId: takeId,
      );
    } on Revision3VoiceTakePreviewStaleCheckpointException {
      throw const Revision3VoiceTakeMediaQaStaleCheckpointException();
    }

    try {
      final result = await inspectTechnicalPlan(
        expectedProjectId: fresh.projectId,
        expectedProjectRevision: fresh.projectRevision,
        plan: plan,
      );
      if (!_revision3VoiceTakeMediaQaMatches(
        result,
        projectId: fresh.projectId,
        projectRevision: fresh.projectRevision,
        plan: plan,
      )) {
        throw Revision3VoiceTakeMediaQaRequiresReopenException(
          cause: StateError(
            'Voice media QA receipt disagrees with the fresh checkpoint',
          ),
        );
      }
      return result;
    } on Revision3VoiceTakeMediaQaStaleCheckpointException {
      rethrow;
    } on Revision3VoiceTakeMediaQaRequiresReopenException {
      rethrow;
    } on ModFfiException catch (error) {
      if (_revision3VoiceTakeMediaQaStaleCodes.contains(error.code)) {
        throw const Revision3VoiceTakeMediaQaStaleCheckpointException();
      }
      throw Revision3VoiceTakeMediaQaRequiresReopenException(cause: error);
    } on ManagedProjectReentrantOperationException {
      rethrow;
    } on ManagedProjectSessionException catch (error) {
      throw Revision3VoiceTakeMediaQaRequiresReopenException(cause: error);
    }
  }
}

const _revision3VoiceTakeMediaQaStaleCodes = <String>{
  'AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT',
};

bool _revision3VoiceTakeMediaQaMatches(
  AuthoringRevision3VoiceTakeMediaQaResult result, {
  required String projectId,
  required int projectRevision,
  required Revision3VoiceTakePreviewTechnicalPlan plan,
}) {
  final assuranceMatches = switch (plan.codec) {
    Revision3ContentVoiceOggCodec.vorbis =>
      result.assurance ==
              AuthoringRevision3VoiceTakeMediaAssurance.vorbisFullPcmDecode &&
          result.duration.timebaseHz == plan.sampleRate,
    Revision3ContentVoiceOggCodec.opus =>
      result.assurance ==
              AuthoringRevision3VoiceTakeMediaAssurance
                  .opusPacketAndTimingStructureOnly &&
          result.duration.timebaseHz == 48000,
  };
  return result.projectId == projectId &&
      result.projectRevision == projectRevision &&
      result.lineId == plan.lineId &&
      result.lineRevision == plan.expectedLineRevision &&
      result.localizationId == plan.localizationId &&
      result.localizationRevision == plan.expectedLocalizationRevision &&
      result.locId == plan.locId &&
      result.slotId == plan.slotId &&
      result.slotRevision == plan.expectedSlotRevision &&
      result.locale == plan.locale &&
      result.takeId == plan.takeId &&
      result.takeRevision == plan.expectedTakeRevision &&
      result.asset.sha256 == plan.assetSha256 &&
      result.asset.byteLength == plan.assetByteLength &&
      result.asset.logicalName == plan.assetLogicalName &&
      result.status.name == plan.status.name &&
      result.ogg.codec.name == plan.codec.name &&
      result.ogg.channels == plan.channels &&
      result.ogg.sampleRate == plan.sampleRate &&
      assuranceMatches &&
      result.mediaAuthority ==
          AuthoringRevision3VoiceTakeMediaAuthority
              .exactCurrentManagedCasVoiceTakeMediaQaV1 &&
      result.inspectionScope ==
          AuthoringRevision3VoiceTakeMediaInspectionScope
              .selectedVoiceTakeMediaInputOnly &&
      result.qualityStatus ==
          AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated &&
      result.audibilityStatus ==
          AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated &&
      result.projectWriteStatus ==
          AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed &&
      result.gameWriteStatus ==
          AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed &&
      result.saveWriteStatus ==
          AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed &&
      result.buildStatus ==
          AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated &&
      result.deploymentStatus ==
          AuthoringRevision3VoiceTakeMediaDeploymentStatus.notPerformed &&
      result.runtimeStatus ==
          AuthoringRevision3VoiceTakeMediaRuntimeStatus.notQualified;
}
