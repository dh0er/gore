import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_media_qa_service.dart';

import '../support/revision3_voice_preview_fixture.dart';

void main() {
  test('media QA service reloads and binds the exact Preview plan', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoicePreviewContentIndex(),
    );
    String? receivedProjectId;
    int? receivedRevision;
    String? inspectedLine;
    String? inspectedLocale;
    String? inspectedTake;
    final service = Revision3VoiceTakeMediaQaAuthoringService(
      loadContentIndex: () async => revision3VoicePreviewContentIndex(),
      inspectTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            receivedProjectId = expectedProjectId;
            receivedRevision = expectedProjectRevision;
            inspectedLine = plan.lineId;
            inspectedLocale = plan.locale;
            inspectedTake = plan.takeId;
            final request = revision3VoicePreviewRequest();
            return AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
              revision3VoiceMediaQaResponse(request: request),
              request: request,
            );
          },
    );

    final result = await service.inspect(
      checkpoint: checkpoint,
      lineId: revision3VoicePreviewLineId,
      locale: 'de',
      takeId: revision3VoicePreviewTakeId,
    );

    expect(receivedProjectId, revision3VoicePreviewProjectId);
    expect(receivedRevision, 7);
    expect(inspectedLine, revision3VoicePreviewLineId);
    expect(inspectedLocale, 'de');
    expect(inspectedTake, revision3VoicePreviewTakeId);
    expect(result.duration.sampleFrames, 3840);
  });

  test('media QA service maps catalog and leaf drift to stale', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoicePreviewContentIndex(),
    );
    var inspectorCalls = 0;
    final changedCatalogService = Revision3VoiceTakeMediaQaAuthoringService(
      loadContentIndex: () async =>
          revision3VoicePreviewContentIndex(revision: 8),
      inspectTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            inspectorCalls++;
            throw StateError('must not inspect a stale catalog');
          },
    );

    await expectLater(
      changedCatalogService.inspect(
        checkpoint: checkpoint,
        lineId: revision3VoicePreviewLineId,
        locale: 'de',
        takeId: revision3VoicePreviewTakeId,
      ),
      throwsA(isA<Revision3VoiceTakeMediaQaStaleCheckpointException>()),
    );
    expect(inspectorCalls, 0);

    for (final code in <String>[
      'AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_LOCALIZATION_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_SLOT_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_TAKE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT',
    ]) {
      final service = Revision3VoiceTakeMediaQaAuthoringService(
        loadContentIndex: () async => revision3VoicePreviewContentIndex(),
        inspectTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => throw ModFfiException(
              command: 'authoring_store_inspect_revision3_voice_take_media_v1',
              code: code,
              message: 'fake exact graph drift',
            ),
      );
      await expectLater(
        service.inspect(
          checkpoint: checkpoint,
          lineId: revision3VoicePreviewLineId,
          locale: 'de',
          takeId: revision3VoicePreviewTakeId,
        ),
        throwsA(isA<Revision3VoiceTakeMediaQaStaleCheckpointException>()),
        reason: code,
      );
    }
  });

  test('media QA service maps Store/session uncertainty to reopen', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoicePreviewContentIndex(),
    );
    final errors = <Object>[
      const ModFfiException(
        command: 'authoring_store_inspect_revision3_voice_take_media_v1',
        code: 'AUTHORING_REVISION3_VOICE_MEDIA_STORE_INVARIANT',
        message: 'fake Store uncertainty',
      ),
      const ManagedProjectVerificationException(
        'fake exact session uncertainty',
      ),
    ];

    for (final error in errors) {
      final service = Revision3VoiceTakeMediaQaAuthoringService(
        loadContentIndex: () async => revision3VoicePreviewContentIndex(),
        inspectTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => throw error,
      );
      await expectLater(
        service.inspect(
          checkpoint: checkpoint,
          lineId: revision3VoicePreviewLineId,
          locale: 'de',
          takeId: revision3VoicePreviewTakeId,
        ),
        throwsA(
          isA<Revision3VoiceTakeMediaQaRequiresReopenException>().having(
            (failure) => failure.cause,
            'cause',
            same(error),
          ),
        ),
      );
    }
  });

  test(
    'media QA service fails closed on a valid but mismatched receipt',
    () async {
      final checkpoint = Revision3VoiceCatalog.fromContentIndex(
        revision3VoicePreviewContentIndex(),
      );
      final service = Revision3VoiceTakeMediaQaAuthoringService(
        loadContentIndex: () async => revision3VoicePreviewContentIndex(),
        inspectTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              final request = revision3VoicePreviewRequest();
              return AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
                revision3VoiceMediaQaResponse(
                  request: request,
                  status: 'approved',
                ),
                request: request,
              );
            },
      );

      await expectLater(
        service.inspect(
          checkpoint: checkpoint,
          lineId: revision3VoicePreviewLineId,
          locale: 'de',
          takeId: revision3VoicePreviewTakeId,
        ),
        throwsA(isA<Revision3VoiceTakeMediaQaRequiresReopenException>()),
      );
    },
  );
}
