part of '../core/mod_ffi.dart';

const _authoringRevision3VoiceMediaQaAuthority =
    'exact_current_managed_cas_voice_take_media_qa_v1';
const _authoringRevision3VoiceMediaQaScope =
    'selected_voice_take_media_input_only';

enum AuthoringRevision3VoiceTakeMediaAssurance {
  vorbisFullPcmDecode,
  opusPacketAndTimingStructureOnly,
}

enum AuthoringRevision3VoiceTakeMediaAuthority {
  exactCurrentManagedCasVoiceTakeMediaQaV1,
}

enum AuthoringRevision3VoiceTakeMediaInspectionScope {
  selectedVoiceTakeMediaInputOnly,
}

enum AuthoringRevision3VoiceTakeMediaEvaluationStatus { notEvaluated }

enum AuthoringRevision3VoiceTakeMediaWriteStatus { notPerformed }

enum AuthoringRevision3VoiceTakeMediaDeploymentStatus { notPerformed }

enum AuthoringRevision3VoiceTakeMediaRuntimeStatus { notQualified }

/// Exact rational playback duration. No floating-point duration is accepted
/// from native code or synthesized at this boundary.
final class AuthoringRevision3VoiceTakeMediaDuration {
  const AuthoringRevision3VoiceTakeMediaDuration._({
    required this.sampleFrames,
    required this.timebaseHz,
  });

  final int sampleFrames;
  final int timebaseHz;
}

/// Strict pathless receipt for one exact-current managed VoiceTake media read.
///
/// This result proves only the native media-input checks represented by
/// [assurance]. It contains no materialized file, CAS path, mutation receipt,
/// perceptual-quality claim, audibility claim, or runtime qualification.
final class AuthoringRevision3VoiceTakeMediaQaResult {
  const AuthoringRevision3VoiceTakeMediaQaResult._({
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.locId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.takeId,
    required this.takeRevision,
    required this.asset,
    required this.status,
    required this.ogg,
    required this.duration,
    required this.assurance,
    required this.mediaAuthority,
    required this.inspectionScope,
    required this.qualityStatus,
    required this.audibilityStatus,
    required this.projectWriteStatus,
    required this.gameWriteStatus,
    required this.saveWriteStatus,
    required this.buildStatus,
    required this.deploymentStatus,
    required this.runtimeStatus,
  });

  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String locId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String takeId;
  final int takeRevision;
  final AuthoringRevision3VoiceAsset asset;
  final AuthoringRevision3VoiceTakeStatus status;
  final AuthoringRevision3VoiceOggMetadata ogg;
  final AuthoringRevision3VoiceTakeMediaDuration duration;
  final AuthoringRevision3VoiceTakeMediaAssurance assurance;
  final AuthoringRevision3VoiceTakeMediaAuthority mediaAuthority;
  final AuthoringRevision3VoiceTakeMediaInspectionScope inspectionScope;
  final AuthoringRevision3VoiceTakeMediaEvaluationStatus qualityStatus;
  final AuthoringRevision3VoiceTakeMediaEvaluationStatus audibilityStatus;
  final AuthoringRevision3VoiceTakeMediaWriteStatus projectWriteStatus;
  final AuthoringRevision3VoiceTakeMediaWriteStatus gameWriteStatus;
  final AuthoringRevision3VoiceTakeMediaWriteStatus saveWriteStatus;
  final AuthoringRevision3VoiceTakeMediaEvaluationStatus buildStatus;
  final AuthoringRevision3VoiceTakeMediaDeploymentStatus deploymentStatus;
  final AuthoringRevision3VoiceTakeMediaRuntimeStatus runtimeStatus;

  factory AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'line_id',
      'line_revision',
      'localization_id',
      'localization_revision',
      'loc_id',
      'slot_id',
      'slot_revision',
      'locale',
      'take_id',
      'take_revision',
      'asset',
      'status',
      'ogg',
      'duration_sample_frames',
      'duration_timebase_hz',
      'assurance',
      'media_authority',
      'inspection_scope',
      'quality_status',
      'audibility_status',
      'project_write_status',
      'game_write_status',
      'save_write_status',
      'build_status',
      'deployment_status',
      'runtime_status',
    }, 'revision-3 Voice take media QA response');
    if (json['ok'] != true || json['outcome'] != 'media_qa_complete') {
      throw const FormatException(
        'revision-3 Voice take media QA response is not complete',
      );
    }

    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final asset = _authoringRevision3VoicePreviewAsset(
      json['asset'],
      request.expectedAsset,
    );
    final status = _authoringRevision3VoiceStatus(json['status']);
    final ogg = _authoringRevision3VoiceOgg(json['ogg']);
    final duration = AuthoringRevision3VoiceTakeMediaDuration._(
      sampleFrames: _authoringRequiredInt(
        json,
        'duration_sample_frames',
        min: 1,
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      timebaseHz: _authoringRequiredInt(
        json,
        'duration_timebase_hz',
        min: 1,
        max: 0xffffffff,
      ),
    );
    final assurance = switch (json['assurance']) {
      'vorbis_full_pcm_decode' =>
        AuthoringRevision3VoiceTakeMediaAssurance.vorbisFullPcmDecode,
      'opus_packet_and_timing_structure_only' =>
        AuthoringRevision3VoiceTakeMediaAssurance
            .opusPacketAndTimingStructureOnly,
      _ => throw const FormatException(
        'revision-3 Voice take media QA assurance is unsupported',
      ),
    };
    final codecContractIsExact = switch ((ogg.codec, assurance)) {
      (
        AuthoringRevision3VoiceOggCodec.vorbis,
        AuthoringRevision3VoiceTakeMediaAssurance.vorbisFullPcmDecode,
      ) =>
        duration.timebaseHz == ogg.sampleRate,
      (
        AuthoringRevision3VoiceOggCodec.opus,
        AuthoringRevision3VoiceTakeMediaAssurance
            .opusPacketAndTimingStructureOnly,
      ) =>
        ogg.sampleRate == 48000 && duration.timebaseHz == 48000,
      _ => false,
    };
    if (!codecContractIsExact) {
      throw const FormatException(
        'revision-3 Voice take media QA codec timing or assurance disagrees',
      );
    }

    final result = AuthoringRevision3VoiceTakeMediaQaResult._(
      basisHead: basisHead,
      projectId: _authoringRevision3VoiceEntityId(json, 'project_id'),
      projectRevision: _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      lineId: _authoringRevision3VoiceEntityId(json, 'line_id'),
      lineRevision: _authoringRequiredInt(
        json,
        'line_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      localizationId: _authoringRevision3VoiceEntityId(json, 'localization_id'),
      localizationRevision: _authoringRequiredInt(
        json,
        'localization_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      locId: _authoringRevision3VoiceString(
        json,
        'loc_id',
        maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
      ),
      slotId: _authoringRevision3VoiceEntityId(json, 'slot_id'),
      slotRevision: _authoringRequiredInt(
        json,
        'slot_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      locale: _authoringRevision3VoiceLocale(
        _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
      ),
      takeId: _authoringRevision3VoiceEntityId(json, 'take_id'),
      takeRevision: _authoringRequiredInt(
        json,
        'take_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      asset: asset,
      status: status,
      ogg: ogg,
      duration: duration,
      assurance: assurance,
      mediaAuthority: switch (json['media_authority']) {
        _authoringRevision3VoiceMediaQaAuthority =>
          AuthoringRevision3VoiceTakeMediaAuthority
              .exactCurrentManagedCasVoiceTakeMediaQaV1,
        _ => throw const FormatException(
          'revision-3 Voice take media QA grants invalid media authority',
        ),
      },
      inspectionScope: switch (json['inspection_scope']) {
        _authoringRevision3VoiceMediaQaScope =>
          AuthoringRevision3VoiceTakeMediaInspectionScope
              .selectedVoiceTakeMediaInputOnly,
        _ => throw const FormatException(
          'revision-3 Voice take media QA grants invalid inspection scope',
        ),
      },
      qualityStatus: _authoringRevision3VoiceMediaEvaluationStatus(
        json['quality_status'],
        'quality',
      ),
      audibilityStatus: _authoringRevision3VoiceMediaEvaluationStatus(
        json['audibility_status'],
        'audibility',
      ),
      projectWriteStatus: _authoringRevision3VoiceMediaWriteStatus(
        json['project_write_status'],
        'project',
      ),
      gameWriteStatus: _authoringRevision3VoiceMediaWriteStatus(
        json['game_write_status'],
        'game',
      ),
      saveWriteStatus: _authoringRevision3VoiceMediaWriteStatus(
        json['save_write_status'],
        'save',
      ),
      buildStatus: _authoringRevision3VoiceMediaEvaluationStatus(
        json['build_status'],
        'build',
      ),
      deploymentStatus: switch (json['deployment_status']) {
        'not_performed' =>
          AuthoringRevision3VoiceTakeMediaDeploymentStatus.notPerformed,
        _ => throw const FormatException(
          'revision-3 Voice take media QA grants invalid deployment status',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'not_qualified' =>
          AuthoringRevision3VoiceTakeMediaRuntimeStatus.notQualified,
        _ => throw const FormatException(
          'revision-3 Voice take media QA grants invalid runtime status',
        ),
      },
    );
    if (result.basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        result.projectId != request.expectedProjectId ||
        result.projectRevision != request.expectedRevision ||
        result.lineId != request.lineId ||
        result.lineRevision != request.expectedLineRevision ||
        result.localizationId != request.localizationId ||
        result.localizationRevision != request.expectedLocalizationRevision ||
        result.locId != request.expectedLocId ||
        result.slotId != request.slotId ||
        result.slotRevision != request.expectedSlotRevision ||
        result.locale != request.locale ||
        result.takeId != request.takeId ||
        result.takeRevision != request.expectedTakeRevision ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(result.locId)) {
      throw const FormatException(
        'revision-3 Voice take media QA disagrees with its exact request',
      );
    }
    return result;
  }
}

AuthoringRevision3VoiceTakeMediaEvaluationStatus
_authoringRevision3VoiceMediaEvaluationStatus(Object? value, String context) =>
    switch (value) {
      'not_evaluated' =>
        AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated,
      _ => throw FormatException(
        'revision-3 Voice take media QA grants invalid $context status',
      ),
    };

AuthoringRevision3VoiceTakeMediaWriteStatus
_authoringRevision3VoiceMediaWriteStatus(Object? value, String context) =>
    switch (value) {
      'not_performed' =>
        AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed,
      _ => throw FormatException(
        'revision-3 Voice take media QA grants invalid $context write status',
      ),
    };
