import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:path/path.dart' as p;

import 'revision3_voice_content_fixture.dart';

const revision3VoicePreviewProjectId = '11111111111111111111111111111111';
const revision3VoicePreviewLineId = '33333333333333333333333333333333';
const revision3VoicePreviewLocalizationId = '22222222222222222222222222222222';
const revision3VoicePreviewSlotId = '44444444444444444444444444444444';
const revision3VoicePreviewTakeId = '55000000000000000000000000000000';
const revision3VoicePreviewAssetSha256 =
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
const revision3VoicePreviewAssetByteLength = 3;
const revision3VoicePreviewLogicalName = 'GRD_263_ASGHAN_OPEN_INFO_06_02.ogg';
const revision3VoicePreviewLocId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
const revision3VoicePreviewBytes = <int>[0x61, 0x62, 0x63];
const revision3VoicePreviewCleanupToken =
    '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';

/// Create a native-shaped preview root and guarantee test-process cleanup even
/// when an assertion interrupts the fake capability lifecycle.
Future<Directory> createRevision3VoicePreviewTestRoot() async {
  final random = Random.secure();
  for (var attempt = 0; attempt < 8; attempt++) {
    final suffix = List<int>.generate(
      32,
      (_) => random.nextInt(256),
    ).map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
    final root = Directory(
      p.join(
        Directory.systemTemp.path,
        'gore-mod-studio-voice-preview-$suffix',
      ),
    );
    try {
      await root.create();
      addTearDown(() async {
        if (await root.exists()) await root.delete(recursive: true);
      });
      return root;
    } on FileSystemException {
      // A cryptographic-name collision is harmless; retry with a fresh name.
    }
  }
  throw StateError('could not create a unique fake Voice preview root');
}

Revision3ContentIndex revision3VoicePreviewContentIndex({int revision = 7}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 1,
  );
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final take = entities.singleWhere(
    (entity) => entity['id'] == revision3VoicePreviewTakeId,
  );
  take['asset_references'] = <Object?>[
    <String, Object?>{
      'role': 'voice_audio',
      'sha256': revision3VoicePreviewAssetSha256,
      'byte_len': revision3VoicePreviewAssetByteLength,
      'logical_name': revision3VoicePreviewLogicalName,
      'expected_media_type': 'audio/ogg',
      'resolution': 'resolved',
    },
  ];
  json['assets'] = <Object?>[
    <String, Object?>{
      'sha256': revision3VoicePreviewAssetSha256,
      'byte_len': revision3VoicePreviewAssetByteLength,
      'media_type': 'audio/ogg',
      'class': 'voice_audio',
    },
  ];
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3VoiceTakePreviewTechnicalPlan revision3VoicePreviewPlan({
  int revision = 7,
}) => Revision3VoiceTakePreviewTechnicalPlan.forCheckpoint(
  catalog: Revision3VoiceCatalog.fromContentIndex(
    revision3VoicePreviewContentIndex(revision: revision),
  ),
  lineId: revision3VoicePreviewLineId,
  locale: 'de',
  takeId: revision3VoicePreviewTakeId,
);

AuthoringWorkingHead revision3VoicePreviewHead({String sealByte = 'b'}) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': 321,
          'sha256': List<String>.filled(64, sealByte).join(),
        },
      }),
    );

AuthoringRevision3VoiceTakePreviewRequestV1 revision3VoicePreviewRequest({
  AuthoringWorkingHead? head,
}) => AuthoringRevision3VoiceTakePreviewRequestV1(
  expectedHead: head ?? revision3VoicePreviewHead(),
  expectedProjectId: revision3VoicePreviewProjectId,
  expectedRevision: 7,
  lineId: revision3VoicePreviewLineId,
  expectedLineRevision: 2,
  localizationId: revision3VoicePreviewLocalizationId,
  expectedLocalizationRevision: 0,
  expectedLocId: revision3VoicePreviewLocId,
  slotId: revision3VoicePreviewSlotId,
  expectedSlotRevision: 1,
  locale: 'de',
  takeId: revision3VoicePreviewTakeId,
  expectedTakeRevision: 0,
  expectedAsset: const AuthoringRevision3VoiceTakePreviewExpectedAsset(
    sha256: revision3VoicePreviewAssetSha256,
    byteLength: revision3VoicePreviewAssetByteLength,
    logicalName: revision3VoicePreviewLogicalName,
  ),
);

Map<String, Object?> revision3VoicePreviewResponse({
  required String previewRoot,
  String cleanupToken = revision3VoicePreviewCleanupToken,
  AuthoringRevision3VoiceTakePreviewRequestV1? request,
}) {
  final exactRequest = request ?? revision3VoicePreviewRequest();
  return <String, Object?>{
    'ok': true,
    'outcome': 'preview_ready',
    'basis_head_json': exactRequest.expectedHead.canonicalJson,
    'project_id': exactRequest.expectedProjectId,
    'project_revision': exactRequest.expectedRevision,
    'line_id': exactRequest.lineId,
    'line_revision': exactRequest.expectedLineRevision,
    'localization_id': exactRequest.localizationId,
    'localization_revision': exactRequest.expectedLocalizationRevision,
    'loc_id': exactRequest.expectedLocId,
    'slot_id': exactRequest.slotId,
    'slot_revision': exactRequest.expectedSlotRevision,
    'locale': exactRequest.locale,
    'take_id': exactRequest.takeId,
    'take_revision': exactRequest.expectedTakeRevision,
    'asset': <String, Object?>{
      'sha256': exactRequest.expectedAsset.sha256,
      'byte_len': exactRequest.expectedAsset.byteLength,
      'logical_name': exactRequest.expectedAsset.logicalName,
    },
    'status': 'recorded',
    'ogg': <String, Object?>{
      'codec': 'vorbis',
      'channels': 1,
      'sample_rate': 48000,
      'pages': 1,
      'logical_streams': 1,
    },
    'preview_path': p.join(previewRoot, 'preview.ogg'),
    'preview_leaf': 'preview.ogg',
    'preview_authority': 'exact_current_managed_cas_voice_take_v1',
    'cleanup_token': cleanupToken,
    'preview_lifecycle': 'native_opaque_cleanup_capability_v1',
    'project_write_status': 'not_performed',
    'game_write_status': 'not_performed',
    'save_write_status': 'not_performed',
    'build_status': 'not_performed',
    'deployment_status': 'not_performed',
    'runtime_status': 'not_qualified',
  };
}

Map<String, Object?> revision3VoiceMediaQaResponse({
  AuthoringRevision3VoiceTakePreviewRequestV1? request,
  String status = 'recorded',
  String codec = 'vorbis',
  int channels = 1,
  int sampleRate = 48000,
  int pages = 3,
  int logicalStreams = 1,
  int durationSampleFrames = 3840,
  int? durationTimebaseHz,
  String? assurance,
}) {
  final exactRequest = request ?? revision3VoicePreviewRequest();
  final isOpus = codec == 'opus';
  return <String, Object?>{
    'ok': true,
    'outcome': 'media_qa_complete',
    'basis_head_json': exactRequest.expectedHead.canonicalJson,
    'project_id': exactRequest.expectedProjectId,
    'project_revision': exactRequest.expectedRevision,
    'line_id': exactRequest.lineId,
    'line_revision': exactRequest.expectedLineRevision,
    'localization_id': exactRequest.localizationId,
    'localization_revision': exactRequest.expectedLocalizationRevision,
    'loc_id': exactRequest.expectedLocId,
    'slot_id': exactRequest.slotId,
    'slot_revision': exactRequest.expectedSlotRevision,
    'locale': exactRequest.locale,
    'take_id': exactRequest.takeId,
    'take_revision': exactRequest.expectedTakeRevision,
    'asset': <String, Object?>{
      'sha256': exactRequest.expectedAsset.sha256,
      'byte_len': exactRequest.expectedAsset.byteLength,
      'logical_name': exactRequest.expectedAsset.logicalName,
    },
    'status': status,
    'ogg': <String, Object?>{
      'codec': codec,
      'channels': channels,
      'sample_rate': sampleRate,
      'pages': pages,
      'logical_streams': logicalStreams,
    },
    'duration_sample_frames': durationSampleFrames,
    'duration_timebase_hz': durationTimebaseHz ?? (isOpus ? 48000 : sampleRate),
    'assurance':
        assurance ??
        (isOpus
            ? 'opus_packet_and_timing_structure_only'
            : 'vorbis_full_pcm_decode'),
    'media_authority': 'exact_current_managed_cas_voice_take_media_qa_v1',
    'inspection_scope': 'selected_voice_take_media_input_only',
    'quality_status': 'not_evaluated',
    'audibility_status': 'not_evaluated',
    'project_write_status': 'not_performed',
    'game_write_status': 'not_performed',
    'save_write_status': 'not_performed',
    'build_status': 'not_evaluated',
    'deployment_status': 'not_performed',
    'runtime_status': 'not_qualified',
  };
}

Map<String, Object?> revision3VoicePreviewRegistrationResponse({
  required String previewRoot,
  String cleanupToken = revision3VoicePreviewCleanupToken,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'preview_capability_registered',
  'cleanup_token': cleanupToken,
  'preview_root': previewRoot,
  'preview_path': p.join(previewRoot, 'preview.ogg'),
  'preview_leaf': 'preview.ogg',
  'preview_authority': 'native_owned_ephemeral_temp_capability_v1',
  'preview_lifecycle': 'native_opaque_cleanup_capability_v1',
  'project_write_status': 'not_performed',
  'game_write_status': 'not_performed',
  'save_write_status': 'not_performed',
  'build_status': 'not_performed',
  'deployment_status': 'not_performed',
  'runtime_status': 'not_qualified',
};

Map<String, Object?> revision3VoicePreviewCleanupResponse() =>
    <String, Object?>{
      'ok': true,
      'outcome': 'preview_cleanup_complete',
      'cleanup_status': 'performed',
      'project_write_status': 'not_performed',
      'game_write_status': 'not_performed',
      'save_write_status': 'not_performed',
      'build_status': 'not_performed',
      'deployment_status': 'not_performed',
      'runtime_status': 'not_qualified',
    };
