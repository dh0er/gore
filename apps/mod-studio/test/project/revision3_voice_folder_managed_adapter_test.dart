import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_folder_authoring.dart';
import 'package:gore_mod/project/revision3_voice_folder_managed_adapter.dart';

import '../support/revision3_voice_content_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '_revision3_voice_batch_test_support.dart';

const _locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
const _contentTakeId = '55000000000000000000000000000000';
const _sourceFolder = r'C:\Recordings\German';

void main() {
  group('native plan presentation', () {
    test(
      'maps ready facts through the exact index without exposing authority',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create(
          existingSlot: true,
        );
        final nativePlan = fixture.plan();
        final capture = _AdapterCapture(nativePlan: nativePlan);
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: 7,
          head: fixture.basisHead,
          index: _nativeIndex(
            revision: 7,
            existingSlot: true,
            candidateCount: 0,
            slotId: voiceBatchTestExistingSlotId,
          ),
          capture: capture,
        );

        final plan = await adapter.service.plan(
          _request(head: fixture.basisHead),
        );

        expect(capture.planCalls, 1);
        expect(capture.plannedFolder, _sourceFolder);
        expect(capture.plannedLocale, voiceBatchTestLocale);
        expect(plan.folderLabel, 'German');
        expect(plan.counts.scanned, 1);
        expect(plan.counts.ogg, 1);
        expect(plan.counts.ready, 1);
        expect(plan.counts.alreadyPresent, 0);
        expect(plan.counts.blocked, 0);
        expect(plan.counts.ignored, 0);
        expect(plan.canApply, isTrue);
        expect(plan.changesDialogText, isFalse);
        expect(plan.changesSelection, isFalse);

        final row = plan.rows.single;
        expect(row.status, Revision3VoiceFolderRowStatus.ready);
        expect(row.codec, Revision3VoiceFolderCodec.vorbis);
        expect(row.byteLength, 100);
        expect(row.lineLabel, 'Asghan greeting');
        expect(row.speakerLabel, 'Asghan');
        expect(row.beforeTakeCount, 0);
        expect(row.afterTakeCount, 1);
        expect(row.targetState, Revision3VoiceFolderTargetState.unresolved);
        expect(row.importedTakeWillBeSelected, isFalse);
        expect(row.changesDialogText, isFalse);
        _expectNoAuthorityLabels(
          plan,
          forbidden: <String>{
            revision3VoiceFixtureProjectId,
            revision3VoiceFixtureLocalizationId,
            revision3VoiceFixtureLineId,
            voiceBatchTestExistingSlotId,
            revision3VoiceFixtureTakeId,
            revision3VoiceFixtureAssetSha256,
            voiceBatchTestManifestSha,
            voiceBatchTestPlanSha,
            _locId,
            _sourceFolder,
          },
        );
      },
    );

    test(
      'maps already-present count and exact existing target state',
      () async {
        final existing = _existingPlan();
        final capture = _AdapterCapture(nativePlan: existing.plan);
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: existing.revision,
          head: existing.head,
          index: _nativeIndex(
            revision: existing.revision,
            existingSlot: true,
            candidateCount: 1,
            slotId: revision3VoiceFixtureSlotId,
            takeId: revision3VoiceFixtureTakeId,
          ),
          capture: capture,
        );

        final plan = await adapter.service.plan(
          _request(head: existing.head, revision: existing.revision),
        );

        expect(plan.counts.scanned, 2);
        expect(plan.counts.ogg, 1);
        expect(plan.counts.ready, 0);
        expect(plan.counts.alreadyPresent, 1);
        expect(plan.counts.blocked, 0);
        expect(plan.counts.ignored, 1);
        expect(plan.canApply, isFalse);
        final row = plan.rows.single;
        expect(row.status, Revision3VoiceFolderRowStatus.alreadyPresent);
        expect(row.beforeTakeCount, 1);
        expect(row.afterTakeCount, 1);
        expect(row.targetState, Revision3VoiceFolderTargetState.unresolved);
        expect(row.takeDisplayName, 'Asghan take');
        expect(row.selectionUnchanged, isTrue);
        expect(row.targetUnchanged, isTrue);
      },
    );

    test(
      'maps all blocked classes and preserves ignored scan counts',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final nativePlan = _blockedPlan(fixture);
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: 7,
          head: fixture.basisHead,
          index: _nativeIndex(revision: 7, existingSlot: false),
          capture: _AdapterCapture(nativePlan: nativePlan),
        );

        final plan = await adapter.service.plan(
          _request(head: fixture.basisHead),
        );

        expect(plan.counts.scanned, 5);
        expect(plan.counts.ogg, 3);
        expect(plan.counts.ready, 0);
        expect(plan.counts.alreadyPresent, 0);
        expect(plan.counts.unmatched, 1);
        expect(plan.counts.ambiguous, 1);
        expect(plan.counts.invalid, 1);
        expect(plan.counts.blocked, 3);
        expect(plan.counts.ignored, 2);
        expect(plan.canApply, isFalse);
        expect(
          plan.rows.map((row) => row.status),
          orderedEquals(<Revision3VoiceFolderRowStatus>[
            Revision3VoiceFolderRowStatus.ambiguous,
            Revision3VoiceFolderRowStatus.invalid,
            Revision3VoiceFolderRowStatus.unmatched,
          ]),
        );
        expect(
          plan.rows.first.targetState,
          Revision3VoiceFolderTargetState.ambiguous,
        );
        expect(plan.rows[1].beforeTakeCount, isNull);
        expect(
          plan.rows.last.targetState,
          Revision3VoiceFolderTargetState.unresolved,
        );
        _expectNoAuthorityLabels(
          plan,
          forbidden: <String>{
            revision3VoiceFixtureProjectId,
            revision3VoiceFixtureLocalizationId,
            revision3VoiceFixtureLineId,
            revision3VoiceFixtureSlotId,
            revision3VoiceFixtureTakeId,
            revision3VoiceFixtureAssetSha256,
            voiceBatchTestManifestSha,
            voiceBatchTestPlanSha,
            _locId,
            _sourceFolder,
          },
        );
      },
    );

    test('sanitizes internal-looking exact project labels', () async {
      final adversarial = _adversarialReadyPlan();
      final adapter = _adapter(
        projectId: revision3VoiceFixtureProjectId,
        revision: 7,
        head: adversarial.head,
        index: _nativeIndex(
          revision: 7,
          existingSlot: false,
          lineDisplayName: revision3VoiceFixtureLineId,
          speaker: _locId,
        ),
        capture: _AdapterCapture(nativePlan: adversarial.plan),
      );

      final plan = await adapter.service.plan(_request(head: adversarial.head));

      _expectNoAuthorityLabels(
        plan,
        forbidden: <String>{
          revision3VoiceFixtureProjectId,
          revision3VoiceFixtureLocalizationId,
          revision3VoiceFixtureLineId,
          revision3VoiceFixtureSlotId,
          revision3VoiceFixtureTakeId,
          revision3VoiceFixtureAssetSha256,
          voiceBatchTestManifestSha,
          voiceBatchTestPlanSha,
          _locId,
          _sourceFolder,
        },
      );
    });

    test(
      'sanitizes cross-row, project, and snapshot authority tokens',
      () async {
        final adversarial = _crossRowPrivacyPlan();
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: 7,
          head: adversarial.head,
          index: _nativeIndex(
            revision: 7,
            existingSlot: false,
            lineDisplayName: 'Line ${adversarial.crossRowLocId}',
            speaker: 'Speaker $revision3VoiceFixtureProjectId',
          ),
          capture: _AdapterCapture(nativePlan: adversarial.plan),
        );

        final plan = await adapter.service.plan(
          _request(head: adversarial.head),
        );

        expect(plan.rows, hasLength(2));
        final ready = plan.rows.singleWhere((row) => row.isReady);
        expect(ready.lineLabel, isNull);
        expect(ready.speakerLabel, isNull);
        expect(ready.takeDisplayName, isNull);
        _expectNoAuthorityLabels(
          plan,
          forbidden: <String>{
            adversarial.crossRowLocId,
            revision3VoiceFixtureProjectId,
            adversarial.head.snapshotSha256,
          },
        );
      },
    );

    test('sanitizes technical source-folder basenames', () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      for (final basename in <String>[
        _locId,
        revision3VoiceFixtureLineId,
        revision3VoiceFixtureAssetSha256,
      ]) {
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: 7,
          head: fixture.basisHead,
          index: _nativeIndex(revision: 7, existingSlot: false),
          capture: _AdapterCapture(nativePlan: fixture.plan()),
        );

        final plan = await adapter.service.plan(
          _request(
            head: fixture.basisHead,
            folder: 'C:\\Recordings\\$basename',
          ),
        );

        expect(
          plan.folderLabel.toLowerCase(),
          isNot(contains(basename.toLowerCase())),
        );
      }
    });
  });

  group('review authority and publication', () {
    test(
      'authority is identity-bound, one-use, and consumed before await',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final nativePlan = fixture.plan();
        final publishGate = Completer<ManagedRevision3VoiceBatchCheckpoint>();
        var publishCalls = 0;
        final adapter = Revision3VoiceFolderManagedAdapter(
          expectedProjectId: revision3VoiceFixtureProjectId,
          expectedProjectRevision: 7,
          expectedProjectHead: fixture.basisHead.canonicalJson,
          loadContentIndex: () async =>
              _nativeIndex(revision: 7, existingSlot: false),
          planNative: ({required sourceFolder, required locale}) async =>
              nativePlan,
          publishNative: ({required sourceFolder, required plan}) {
            publishCalls++;
            return publishGate.future;
          },
        );
        final reviewed = await adapter.service.plan(
          _request(head: fixture.basisHead),
        );
        final cloned = Revision3VoiceFolderImportPlan(
          projectId: reviewed.projectId,
          projectRevision: reviewed.projectRevision,
          projectHead: reviewed.projectHead,
          checkpointToken: reviewed.checkpointToken,
          planToken: reviewed.planToken,
          folderLabel: reviewed.folderLabel,
          locale: reviewed.locale,
          scannedEntryCount: reviewed.counts.scanned,
          ignoredEntryCount: reviewed.counts.ignored,
          rows: reviewed.rows,
        );

        await expectLater(
          adapter.service.apply(plan: cloned),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(publishCalls, 0);

        final firstApply = adapter.service.apply(plan: reviewed);
        await expectLater(
          adapter.service.apply(plan: reviewed),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(publishCalls, 1);
        publishGate.completeError(StateError('publication result unknown'));
        await expectLater(
          firstApply,
          throwsA(isA<Revision3VoiceFolderPublicationUncertainException>()),
        );
        await expectLater(
          adapter.service.apply(plan: reviewed),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(publishCalls, 1);
      },
    );

    test(
      'newest plan invocation retains authority across late completion',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final firstNative = fixture.plan();
        final secondNative = _resealedPlan(
          fixture,
          manifestSha:
              'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
          planSha:
              'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
        );
        final firstGate = Completer<AuthoringRevision3VoiceBatchPlanResult>();
        final secondGate = Completer<AuthoringRevision3VoiceBatchPlanResult>();
        var planCalls = 0;
        var publishCalls = 0;
        AuthoringRevision3VoiceBatchPlanResult? publishedPlan;
        final adapter = Revision3VoiceFolderManagedAdapter(
          expectedProjectId: revision3VoiceFixtureProjectId,
          expectedProjectRevision: 7,
          expectedProjectHead: fixture.basisHead.canonicalJson,
          loadContentIndex: () async =>
              _nativeIndex(revision: 7, existingSlot: false),
          planNative: ({required sourceFolder, required locale}) {
            planCalls++;
            return planCalls == 1 ? firstGate.future : secondGate.future;
          },
          publishNative: ({required sourceFolder, required plan}) async {
            publishCalls++;
            publishedPlan = plan;
            throw const ModFfiException(
              command: 'authoring_store_prepare_revision3_voice_batch_v1',
              code: 'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED',
              message: 'safe fake drift',
            );
          },
        );

        final firstFuture = adapter.service.plan(
          _request(head: fixture.basisHead, folder: r'C:\Recordings\Old'),
        );
        final secondFuture = adapter.service.plan(
          _request(head: fixture.basisHead, folder: r'C:\Recordings\Current'),
        );
        secondGate.complete(secondNative);
        final secondReview = await secondFuture;
        firstGate.complete(firstNative);
        await expectLater(
          firstFuture,
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );

        await expectLater(
          adapter.service.apply(plan: secondReview),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(publishCalls, 1);
        expect(publishedPlan, same(secondNative));
      },
    );

    test('newest authority also survives an older late index read', () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final firstNative = fixture.plan();
      final secondNative = _resealedPlan(
        fixture,
        manifestSha:
            'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
        planSha:
            'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
      );
      final firstIndexGate = Completer<Revision3ContentIndex>();
      final secondIndexGate = Completer<Revision3ContentIndex>();
      var planCalls = 0;
      var indexCalls = 0;
      var publishCalls = 0;
      AuthoringRevision3VoiceBatchPlanResult? publishedPlan;
      final adapter = Revision3VoiceFolderManagedAdapter(
        expectedProjectId: revision3VoiceFixtureProjectId,
        expectedProjectRevision: 7,
        expectedProjectHead: fixture.basisHead.canonicalJson,
        loadContentIndex: () {
          indexCalls++;
          return indexCalls == 1
              ? firstIndexGate.future
              : secondIndexGate.future;
        },
        planNative: ({required sourceFolder, required locale}) async {
          planCalls++;
          return planCalls == 1 ? firstNative : secondNative;
        },
        publishNative: ({required sourceFolder, required plan}) async {
          publishCalls++;
          publishedPlan = plan;
          throw const ModFfiException(
            command: 'authoring_store_prepare_revision3_voice_batch_v1',
            code: 'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED',
            message: 'safe fake drift',
          );
        },
      );

      final firstFuture = adapter.service.plan(
        _request(head: fixture.basisHead, folder: r'C:\Recordings\Old'),
      );
      await Future<void>.delayed(Duration.zero);
      expect(indexCalls, 1);
      final secondFuture = adapter.service.plan(
        _request(head: fixture.basisHead, folder: r'C:\Recordings\Current'),
      );
      await Future<void>.delayed(Duration.zero);
      expect(indexCalls, 2);

      secondIndexGate.complete(_nativeIndex(revision: 7, existingSlot: false));
      final secondReview = await secondFuture;
      firstIndexGate.complete(_nativeIndex(revision: 7, existingSlot: false));
      await expectLater(
        firstFuture,
        throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
      );
      await expectLater(
        adapter.service.apply(plan: secondReview),
        throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
      );
      expect(publishCalls, 1);
      expect(publishedPlan, same(secondNative));
    });

    test(
      'publishes one exact revision and returns deterministic rebind tokens',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final nativePlan = fixture.plan();
        final checkpoint = await _publishedCheckpoint(fixture);
        var publishCalls = 0;
        String? publishedFolder;
        AuthoringRevision3VoiceBatchPlanResult? publishedPlan;
        final adapter = Revision3VoiceFolderManagedAdapter(
          expectedProjectId: revision3VoiceFixtureProjectId,
          expectedProjectRevision: 7,
          expectedProjectHead: fixture.basisHead.canonicalJson,
          loadContentIndex: () async =>
              _nativeIndex(revision: 7, existingSlot: false),
          planNative: ({required sourceFolder, required locale}) async =>
              nativePlan,
          publishNative: ({required sourceFolder, required plan}) async {
            publishCalls++;
            publishedFolder = sourceFolder;
            publishedPlan = plan;
            return checkpoint;
          },
        );
        final reviewed = await adapter.service.plan(
          _request(head: fixture.basisHead),
        );

        final publication = await adapter.service.apply(plan: reviewed);

        expect(publishCalls, 1);
        expect(publishedFolder, _sourceFolder);
        expect(publishedPlan, same(nativePlan));
        expect(publication.projectId, revision3VoiceFixtureProjectId);
        expect(publication.projectRevision, 8);
        expect(publication.projectHead, fixture.candidateHead.canonicalJson);
        expect(publication.planToken, voiceBatchTestPlanSha);
        expect(publication.importedCount, 1);
        expect(
          publication.checkpointToken,
          Revision3VoiceFolderManagedAdapter.checkpointTokenForHead(
            fixture.candidateHead.canonicalJson,
          ),
        );
        expect(publication.checkpointToken, isNot(reviewed.checkpointToken));
      },
    );
  });

  group('staleness and failure translation', () {
    test(
      'rejects same-revision slot-state drift before presenting counts',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final adapter = _adapter(
          projectId: revision3VoiceFixtureProjectId,
          revision: 7,
          head: fixture.basisHead,
          index: _nativeIndex(
            revision: 7,
            existingSlot: true,
            candidateCount: 2,
            slotId: revision3VoiceFixtureSlotId,
          ),
          capture: _AdapterCapture(nativePlan: fixture.plan()),
        );

        await expectLater(
          adapter.service.plan(_request(head: fixture.basisHead)),
          throwsA(isA<Revision3VoiceFolderRequiresReopenException>()),
        );
      },
    );

    test(
      'rejects stale request and exact-index drift without publication',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final nativePlan = fixture.plan();
        var planCalls = 0;
        var publishCalls = 0;
        final adapter = Revision3VoiceFolderManagedAdapter(
          expectedProjectId: revision3VoiceFixtureProjectId,
          expectedProjectRevision: 7,
          expectedProjectHead: fixture.basisHead.canonicalJson,
          loadContentIndex: () async => _nativeIndex(
            projectId: 'ffffffffffffffffffffffffffffffff',
            revision: 7,
            existingSlot: false,
          ),
          planNative: ({required sourceFolder, required locale}) async {
            planCalls++;
            return nativePlan;
          },
          publishNative: ({required sourceFolder, required plan}) async {
            publishCalls++;
            throw StateError('must not publish');
          },
        );

        await expectLater(
          adapter.service.plan(
            _request(
              head: fixture.basisHead,
              projectId: 'ffffffffffffffffffffffffffffffff',
            ),
          ),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(planCalls, 0);

        await expectLater(
          adapter.service.plan(_request(head: fixture.basisHead)),
          throwsA(isA<Revision3VoiceFolderStaleCheckpointException>()),
        );
        expect(planCalls, 1);
        expect(publishCalls, 0);
      },
    );

    test(
      'distinguishes safe prepublication, reopen, and uncertain failures',
      () async {
        final fixture = Revision3VoiceBatchTestFixture.create();
        final nativePlan = fixture.plan();

        Future<void> expectTranslation(Object error, Matcher matcher) async {
          var calls = 0;
          final adapter = Revision3VoiceFolderManagedAdapter(
            expectedProjectId: revision3VoiceFixtureProjectId,
            expectedProjectRevision: 7,
            expectedProjectHead: fixture.basisHead.canonicalJson,
            loadContentIndex: () async =>
                _nativeIndex(revision: 7, existingSlot: false),
            planNative: ({required sourceFolder, required locale}) async =>
                nativePlan,
            publishNative: ({required sourceFolder, required plan}) async {
              calls++;
              throw error;
            },
          );
          final reviewed = await adapter.service.plan(
            _request(head: fixture.basisHead),
          );
          await expectLater(
            adapter.service.apply(plan: reviewed),
            throwsA(matcher),
          );
          expect(calls, 1);
        }

        await expectTranslation(
          const ModFfiException(
            command: 'authoring_store_prepare_revision3_voice_batch_v1',
            code: 'AUTHORING_REVISION3_VOICE_BATCH_PLAN_CHANGED',
            message: 'known before publication',
          ),
          isA<Revision3VoiceFolderStaleCheckpointException>(),
        );
        await expectTranslation(
          const Revision3VoiceBatchRequiresReopenException(),
          isA<Revision3VoiceFolderPublicationUncertainException>(),
        );
        await expectTranslation(
          const Revision3VoiceFolderRequiresReopenException(),
          isA<Revision3VoiceFolderPublicationUncertainException>(),
        );
        await expectTranslation(
          const FormatException('malformed post-publication receipt'),
          isA<Revision3VoiceFolderPublicationUncertainException>(),
        );
        await expectTranslation(
          const ManagedProjectVerificationException(
            'post-publication verification failed',
          ),
          isA<Revision3VoiceFolderPublicationUncertainException>(),
        );
        await expectTranslation(
          const ManagedProjectHeadConflictException(
            'publication did not replace the expected head',
          ),
          isA<Revision3VoiceFolderRequiresReopenException>(),
        );
        await expectTranslation(
          StateError('publication completion unknown'),
          isA<Revision3VoiceFolderPublicationUncertainException>(),
        );
      },
    );
  });
}

Revision3VoiceFolderManagedAdapter _adapter({
  required String projectId,
  required int revision,
  required AuthoringWorkingHead head,
  required Revision3ContentIndex index,
  required _AdapterCapture capture,
}) => Revision3VoiceFolderManagedAdapter(
  expectedProjectId: projectId,
  expectedProjectRevision: revision,
  expectedProjectHead: head.canonicalJson,
  loadContentIndex: () async => index,
  planNative: capture.plan,
  publishNative: capture.publish,
);

final class _AdapterCapture {
  _AdapterCapture({required this.nativePlan});

  final AuthoringRevision3VoiceBatchPlanResult nativePlan;
  int planCalls = 0;
  int publishCalls = 0;
  String? plannedFolder;
  String? plannedLocale;

  Future<AuthoringRevision3VoiceBatchPlanResult> plan({
    required String sourceFolder,
    required String locale,
  }) async {
    planCalls++;
    plannedFolder = sourceFolder;
    plannedLocale = locale;
    return nativePlan;
  }

  Future<ManagedRevision3VoiceBatchCheckpoint> publish({
    required String sourceFolder,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) async {
    publishCalls++;
    throw StateError('publish was not expected');
  }
}

Revision3VoiceFolderPlanRequest _request({
  required AuthoringWorkingHead head,
  int revision = 7,
  String projectId = revision3VoiceFixtureProjectId,
  String folder = _sourceFolder,
}) => Revision3VoiceFolderPlanRequest(
  folderPath: folder,
  locale: voiceBatchTestLocale,
  expectedProjectId: projectId,
  expectedProjectRevision: revision,
  expectedProjectHead: head.canonicalJson,
  expectedCheckpointToken:
      Revision3VoiceFolderManagedAdapter.checkpointTokenForHead(
        head.canonicalJson,
      ),
);

Revision3ContentIndex _nativeIndex({
  String projectId = revision3VoiceFixtureProjectId,
  required int revision,
  required bool existingSlot,
  int candidateCount = 0,
  String slotId = revision3VoiceFixtureSlotId,
  String takeId = revision3VoiceFixtureTakeId,
  String lineDisplayName = 'Asghan greeting',
  String speaker = 'Asghan',
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingSlot,
    existingSlotCandidateCount: candidateCount,
    lineDisplayName: lineDisplayName,
    speaker: speaker,
  );
  return Revision3ContentIndex.fromJsonObject(
    _replaceStrings(json, <String, String>{
          revision3VoiceContentProjectId: projectId,
          revision3VoiceContentLocalizationId:
              revision3VoiceFixtureLocalizationId,
          revision3VoiceContentLineId: revision3VoiceFixtureLineId,
          revision3VoiceContentSlotId: slotId,
          _contentTakeId: takeId,
        })
        as Map<String, Object?>,
  );
}

Object? _replaceStrings(Object? value, Map<String, String> replacements) =>
    switch (value) {
      String text => replacements[text] ?? text,
      List<Object?> values => <Object?>[
        for (final item in values) _replaceStrings(item, replacements),
      ],
      Map values => <String, Object?>{
        for (final entry in values.entries)
          entry.key as String: _replaceStrings(entry.value, replacements),
      },
      _ => value,
    };

void _expectNoAuthorityLabels(
  Revision3VoiceFolderImportPlan plan, {
  required Set<String> forbidden,
}) {
  final labels = <String>[
    plan.folderLabel,
    for (final row in plan.rows) ...<String>[
      if (row.lineLabel != null) row.lineLabel!,
      if (row.speakerLabel != null) row.speakerLabel!,
      if (row.takeDisplayName != null) row.takeDisplayName!,
    ],
  ];
  for (final label in labels) {
    for (final secret in forbidden) {
      expect(
        label.toLowerCase(),
        isNot(contains(secret.toLowerCase())),
        reason: 'presentation label "$label" leaks "$secret"',
      );
    }
    expect(label, isNot(contains(r'C:\')));
  }
}

({
  AuthoringRevision3VoiceBatchPlanResult plan,
  AuthoringWorkingHead head,
  int revision,
})
_existingPlan() {
  final projectJson = revision3VoiceFixtureProjectWithExistingSlotJson(
    candidateCount: 1,
  );
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final revision = project['revision']! as int;
  final head = voiceBatchHeadFor(projectJson);
  final item = <String, Object?>{
    'source_name': voiceBatchTestSourceName,
    'status': 'already_present',
    'line_display_name': 'Asghan greeting',
    'speaker': 'Asghan',
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'loc_id': _locId,
    'slot_id': revision3VoiceFixtureSlotId,
    'take_id': revision3VoiceFixtureTakeId,
    'slot_created': false,
    'voice_request_json': null,
    'asset': _asset(voiceBatchTestSourceName),
    'ogg': _ogg(),
  };
  final response = _planResponse(
    head: head,
    revision: revision,
    status: 'no_changes',
    scanned: 2,
    ignored: 1,
    ready: 0,
    alreadyPresent: 1,
    blocked: 0,
    items: <Object?>[item],
  );
  return (
    plan: AuthoringRevision3VoiceBatchPlanResult.fromJson(
      response,
      expectedHead: head,
      currentProjectJson: projectJson,
      expectedLocale: voiceBatchTestLocale,
    ),
    head: head,
    revision: revision,
  );
}

AuthoringRevision3VoiceBatchPlanResult _blockedPlan(
  Revision3VoiceBatchTestFixture fixture,
) {
  Map<String, Object?> blockedItem(
    String sourceName,
    String status, {
    required bool target,
  }) => <String, Object?>{
    'source_name': sourceName,
    'status': status,
    'line_display_name': target ? 'Asghan greeting' : null,
    'speaker': target ? 'Asghan' : null,
    'line_id': target ? revision3VoiceFixtureLineId : null,
    'localization_id': target ? revision3VoiceFixtureLocalizationId : null,
    'loc_id': target ? _locId : null,
    'slot_id': null,
    'take_id': null,
    'slot_created': null,
    'voice_request_json': null,
    'asset': _asset(sourceName),
    'ogg': _ogg(),
  };

  return AuthoringRevision3VoiceBatchPlanResult.fromJson(
    _planResponse(
      head: fixture.basisHead,
      revision: 7,
      status: 'blocked',
      scanned: 5,
      ignored: 2,
      ready: 0,
      alreadyPresent: 0,
      blocked: 3,
      items: <Object?>[
        blockedItem('ambiguous.ogg', 'ambiguous', target: false),
        blockedItem('invalid.ogg', 'target_blocked', target: true),
        blockedItem('unmatched.ogg', 'unmatched', target: false),
      ],
    ),
    expectedHead: fixture.basisHead,
    currentProjectJson: fixture.projectJson,
    expectedLocale: voiceBatchTestLocale,
  );
}

({AuthoringRevision3VoiceBatchPlanResult plan, AuthoringWorkingHead head})
_adversarialReadyPlan() {
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final line = (entities[revision3VoiceFixtureLineId]! as Map)
      .cast<String, Object?>();
  line['display_name'] = revision3VoiceFixtureLineId;
  final payload = (line['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['speaker_hint'] = _locId;
  final projectJson = jsonEncode(project);
  final head = voiceBatchHeadFor(projectJson);
  final request = AuthoringRevision3VoiceTakeRequestV1.forProject(
    expectedHead: head,
    currentProjectJson: projectJson,
    lineId: revision3VoiceFixtureLineId,
    slotId: revision3VoiceFixtureSlotId,
    takeId: revision3VoiceFixtureTakeId,
    locale: voiceBatchTestLocale,
    takeDisplayName: revision3VoiceFixtureTakeId,
    logicalName: voiceBatchTestSourceName,
    status: AuthoringRevision3VoiceTakeStatus.recorded,
  );
  final item = <String, Object?>{
    'source_name': voiceBatchTestSourceName,
    'status': 'ready',
    'line_display_name': revision3VoiceFixtureLineId,
    'speaker': _locId,
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'loc_id': _locId,
    'slot_id': revision3VoiceFixtureSlotId,
    'take_id': revision3VoiceFixtureTakeId,
    'slot_created': true,
    'voice_request_json': request.canonicalJson,
    'asset': _asset(voiceBatchTestSourceName),
    'ogg': _ogg(),
  };
  final plan = AuthoringRevision3VoiceBatchPlanResult.fromJson(
    _planResponse(
      head: head,
      revision: 7,
      status: 'ready',
      scanned: 1,
      ignored: 0,
      ready: 1,
      alreadyPresent: 0,
      blocked: 0,
      items: <Object?>[item],
    ),
    expectedHead: head,
    currentProjectJson: projectJson,
    expectedLocale: voiceBatchTestLocale,
  );
  return (plan: plan, head: head);
}

({
  AuthoringRevision3VoiceBatchPlanResult plan,
  AuthoringWorkingHead head,
  String crossRowLocId,
})
_crossRowPrivacyPlan() {
  const crossRowLocId = 'OTHER_ROW_TECHNICAL_LOCID_17';
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final line = (entities[revision3VoiceFixtureLineId]! as Map)
      .cast<String, Object?>();
  line['display_name'] = 'Line $crossRowLocId';
  final payload = (line['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['speaker_hint'] = 'Speaker $revision3VoiceFixtureProjectId';
  final projectJson = jsonEncode(project);
  final head = voiceBatchHeadFor(projectJson);
  final request = AuthoringRevision3VoiceTakeRequestV1.forProject(
    expectedHead: head,
    currentProjectJson: projectJson,
    lineId: revision3VoiceFixtureLineId,
    slotId: revision3VoiceFixtureSlotId,
    takeId: revision3VoiceFixtureTakeId,
    locale: voiceBatchTestLocale,
    takeDisplayName: 'Snapshot ${head.snapshotSha256}',
    logicalName: voiceBatchTestSourceName,
    status: AuthoringRevision3VoiceTakeStatus.recorded,
  );
  final ready = <String, Object?>{
    'source_name': voiceBatchTestSourceName,
    'status': 'ready',
    'line_display_name': 'Line $crossRowLocId',
    'speaker': 'Speaker $revision3VoiceFixtureProjectId',
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'loc_id': _locId,
    'slot_id': revision3VoiceFixtureSlotId,
    'take_id': revision3VoiceFixtureTakeId,
    'slot_created': true,
    'voice_request_json': request.canonicalJson,
    'asset': _asset(voiceBatchTestSourceName),
    'ogg': _ogg(),
  };
  final unmatchedSource = '$crossRowLocId.ogg';
  final unmatched = <String, Object?>{
    'source_name': unmatchedSource,
    'status': 'unmatched',
    'line_display_name': null,
    'speaker': null,
    'line_id': null,
    'localization_id': null,
    'loc_id': null,
    'slot_id': null,
    'take_id': null,
    'slot_created': null,
    'voice_request_json': null,
    'asset': _asset(unmatchedSource),
    'ogg': _ogg(),
  };
  final plan = AuthoringRevision3VoiceBatchPlanResult.fromJson(
    _planResponse(
      head: head,
      revision: 7,
      status: 'blocked',
      scanned: 2,
      ignored: 0,
      ready: 1,
      alreadyPresent: 0,
      blocked: 1,
      items: <Object?>[ready, unmatched],
    ),
    expectedHead: head,
    currentProjectJson: projectJson,
    expectedLocale: voiceBatchTestLocale,
  );
  return (plan: plan, head: head, crossRowLocId: crossRowLocId);
}

AuthoringRevision3VoiceBatchPlanResult _resealedPlan(
  Revision3VoiceBatchTestFixture fixture, {
  required String manifestSha,
  required String planSha,
}) {
  final response = voiceBatchDeepCopy(fixture.planResponse())
    ..['source_manifest_sha256'] = manifestSha
    ..['plan_sha256'] = planSha;
  return AuthoringRevision3VoiceBatchPlanResult.fromJson(
    response,
    expectedHead: fixture.basisHead,
    currentProjectJson: fixture.projectJson,
    expectedLocale: voiceBatchTestLocale,
  );
}

Map<String, Object?> _planResponse({
  required AuthoringWorkingHead head,
  required int revision,
  required String status,
  required int scanned,
  required int ignored,
  required int ready,
  required int alreadyPresent,
  required int blocked,
  required List<Object?> items,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'planned',
  'basis_head_json': head.canonicalJson,
  'project_id': revision3VoiceFixtureProjectId,
  'revision': revision,
  'locale': voiceBatchTestLocale,
  'source_manifest_sha256': voiceBatchTestManifestSha,
  'plan_sha256': voiceBatchTestPlanSha,
  'status': status,
  'scanned_entry_count': scanned,
  'ogg_file_count': items.length,
  'ready_count': ready,
  'already_present_count': alreadyPresent,
  'blocked_count': blocked,
  'ignored_entry_count': ignored,
  'items': items,
  'build_status': 'blocked',
  'runtime_status': 'runtime_unqualified',
  'target_authority': 'not_granted',
  'publication_status': 'not_supported',
};

Map<String, Object?> _asset(String logicalName) => <String, Object?>{
  'sha256': revision3VoiceFixtureAssetSha256,
  'byte_len': 100,
  'logical_name': logicalName,
};

Map<String, Object?> _ogg() => <String, Object?>{
  'codec': 'vorbis',
  'channels': 1,
  'sample_rate': 48000,
  'pages': 3,
  'logical_streams': 1,
};

Future<ManagedRevision3VoiceBatchCheckpoint> _publishedCheckpoint(
  Revision3VoiceBatchTestFixture fixture,
) async {
  final root = await Directory.systemTemp.createTemp(
    'gore_voice_folder_adapter_',
  );
  final session = await ManagedRevision3AuthoringProjectSession.create(
    root: root,
    store: VoiceBatchManagedStore(fixture),
    projectJson: fixture.projectJson,
  );
  try {
    return await session.prepareAndPublishVoiceBatchV1(
      gameRoot: voiceBatchTestGameRoot,
      sourceFolder: voiceBatchTestSourceFolder,
      plan: fixture.plan(),
    );
  } finally {
    await session.close();
    if (await root.exists()) await root.delete(recursive: true);
  }
}
