import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_preview_fixture.dart';

const _root = r'C:\Projects\VoiceMediaQa.goreproj';
const _command = 'authoring_store_inspect_revision3_voice_take_media_v1';

void main() {
  test('handshake includes the sorted Voice media QA command', () {
    expect(requiredStudioCoreCommands, contains(_command));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('transport is pathless and parses exact Vorbis assurance', () async {
    final request = revision3VoicePreviewRequest();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _command: revision3VoiceMediaQaResponse(request: request),
      },
    );

    final result = await ModFfi(core)
        .authoringStoreInspectRevision3VoiceTakeMediaV1(
          root: _root,
          request: request,
        );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload.keys, <String>[
      'root',
      'voice_take_preview_request_json',
    ]);
    expect(core.calls.single.payload, isNot(contains('preview_root')));
    expect(core.calls.single.payload, isNot(contains('output')));
    expect(
      core.calls.single.payload['voice_take_preview_request_json'],
      request.canonicalJson,
    );
    expect(result.basisHead.canonicalJson, request.expectedHead.canonicalJson);
    expect(result.lineId, request.lineId);
    expect(result.locale, request.locale);
    expect(result.takeId, request.takeId);
    expect(result.asset.sha256, request.expectedAsset.sha256);
    expect(result.status, AuthoringRevision3VoiceTakeStatus.recorded);
    expect(result.ogg.codec, AuthoringRevision3VoiceOggCodec.vorbis);
    expect(result.duration.sampleFrames, 3840);
    expect(result.duration.timebaseHz, 48000);
    expect(
      result.assurance,
      AuthoringRevision3VoiceTakeMediaAssurance.vorbisFullPcmDecode,
    );
    expect(
      result.mediaAuthority,
      AuthoringRevision3VoiceTakeMediaAuthority
          .exactCurrentManagedCasVoiceTakeMediaQaV1,
    );
    expect(
      result.inspectionScope,
      AuthoringRevision3VoiceTakeMediaInspectionScope
          .selectedVoiceTakeMediaInputOnly,
    );
    expect(
      result.qualityStatus,
      AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated,
    );
    expect(
      result.audibilityStatus,
      AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated,
    );
    expect(
      result.projectWriteStatus,
      AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed,
    );
    expect(
      result.gameWriteStatus,
      AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed,
    );
    expect(
      result.saveWriteStatus,
      AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed,
    );
    expect(
      result.buildStatus,
      AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated,
    );
    expect(
      result.deploymentStatus,
      AuthoringRevision3VoiceTakeMediaDeploymentStatus.notPerformed,
    );
    expect(
      result.runtimeStatus,
      AuthoringRevision3VoiceTakeMediaRuntimeStatus.notQualified,
    );
  });

  test('Opus receipt is explicitly structural and fixed to 48 kHz', () async {
    final request = revision3VoicePreviewRequest();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _command: revision3VoiceMediaQaResponse(
          request: request,
          codec: 'opus',
          sampleRate: 48000,
          durationSampleFrames: 960,
        ),
      },
    );

    final result = await ModFfi(core)
        .authoringStoreInspectRevision3VoiceTakeMediaV1(
          root: _root,
          request: request,
        );

    expect(result.ogg.codec, AuthoringRevision3VoiceOggCodec.opus);
    expect(result.duration.sampleFrames, 960);
    expect(result.duration.timebaseHz, 48000);
    expect(
      result.assurance,
      AuthoringRevision3VoiceTakeMediaAssurance
          .opusPacketAndTimingStructureOnly,
    );
  });

  test(
    'strict receipt fails closed on binding, media, timing, and authority drift',
    () async {
      final request = revision3VoicePreviewRequest();
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response['basis_head_json'] = revision3VoicePreviewHead(
          sealByte: 'c',
        ).canonicalJson,
        (response) =>
            response['project_id'] = '99999999999999999999999999999999',
        (response) => response['project_revision'] = 8,
        (response) => response['line_id'] = '99999999999999999999999999999999',
        (response) => response['line_revision'] = 3,
        (response) =>
            response['localization_id'] = '99999999999999999999999999999999',
        (response) => response['localization_revision'] = 1,
        (response) => response['loc_id'] = 'OTHER_SAFE_LOC_ID',
        (response) => response['slot_id'] = '99999999999999999999999999999999',
        (response) => response['slot_revision'] = 2,
        (response) => response['locale'] = 'en',
        (response) => response['take_id'] = '99999999999999999999999999999999',
        (response) => response['take_revision'] = 1,
        (response) => response['take_revision'] = -1,
        (response) => response['project_revision'] = 7.0,
        (response) {
          final asset = (response['asset']! as Map).cast<String, Object?>();
          asset['sha256'] = ''.padLeft(64, 'c');
        },
        (response) {
          final asset = (response['asset']! as Map).cast<String, Object?>();
          asset['byte_len'] = 4;
        },
        (response) {
          final asset = (response['asset']! as Map).cast<String, Object?>();
          asset['logical_name'] = 'other.ogg';
        },
        (response) => response['status'] = 'runtime_ready',
        (response) {
          final ogg = (response['ogg']! as Map).cast<String, Object?>();
          ogg['codec'] = 'aac';
        },
        (response) {
          final ogg = (response['ogg']! as Map).cast<String, Object?>();
          ogg['channels'] = 0;
        },
        (response) {
          final ogg = (response['ogg']! as Map).cast<String, Object?>();
          ogg['sample_rate'] = 0;
        },
        (response) {
          final ogg = (response['ogg']! as Map).cast<String, Object?>();
          ogg['pages'] = 0;
        },
        (response) {
          final ogg = (response['ogg']! as Map).cast<String, Object?>();
          ogg['logical_streams'] = 0;
        },
        (response) => response['duration_sample_frames'] = 0,
        (response) => response['duration_sample_frames'] = 0x8000000000000000,
        (response) => response['duration_timebase_hz'] = 44100,
        (response) =>
            response['assurance'] = 'opus_packet_and_timing_structure_only',
        (response) => response['media_authority'] = 'runtime_ready',
        (response) => response['inspection_scope'] = 'whole_project',
        (response) => response['quality_status'] = 'passed',
        (response) => response['audibility_status'] = 'passed',
        (response) => response['project_write_status'] = 'performed',
        (response) => response['game_write_status'] = 'performed',
        (response) => response['save_write_status'] = 'performed',
        (response) => response['build_status'] = 'passed',
        (response) => response['deployment_status'] = 'performed',
        (response) => response['runtime_status'] = 'qualified',
        (response) => response['preview_path'] = r'C:\Temp\leak.ogg',
      ];

      for (var index = 0; index < mutations.length; index++) {
        final response = revision3VoiceMediaQaResponse(request: request);
        mutations[index](response);
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{_command: response},
            ),
          ).authoringStoreInspectRevision3VoiceTakeMediaV1(
            root: _root,
            request: request,
          ),
          throwsA(
            isA<ModFfiException>().having(
              (error) => error.code,
              'code',
              ModFfiException.malformedNativeResponseCode,
            ),
          ),
          reason: 'mutation $index must fail closed',
        );
      }
    },
  );

  test('relative Store root fails locally without exposing it', () async {
    const sensitiveRoot = r'secret\managed-store';
    final core = FakeGoreCoreFfiService(
      responses: const <String, Map<String, Object?>>{},
    );

    try {
      await ModFfi(core).authoringStoreInspectRevision3VoiceTakeMediaV1(
        root: sensitiveRoot,
        request: revision3VoicePreviewRequest(),
      );
      fail('relative Store root must fail preflight');
    } on ArgumentError catch (error) {
      expect(error.toString(), isNot(contains(sensitiveRoot)));
    }
    expect(core.calls, isEmpty);
  });
}
