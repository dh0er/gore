import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_quest_outline_fixture.dart';

const _projectId = 'ffffffffffffffffffffffffffffffff';
const _localizationId = '11111111111111111111111111111111';
const _lineId = '22222222222222222222222222222222';
const _slotId = '33333333333333333333333333333333';
const _takeId = '44444444444444444444444444444444';
const _questId = '55555555555555555555555555555555';
const _moduleId = '66666666666666666666666666666666';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  test('ContentIndex rejects false transcript facts', () {
    final falseCount = _contentIndexJson();
    final quest = ((falseCount['entities']! as List)[4] as Map)
        .cast<String, Object?>();
    final summary = (quest['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['transcript_count'] = 0;
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseCount),
      throwsFormatException,
    );

    final badSlot = _contentIndexJson();
    final badQuest = ((badSlot['entities']! as List)[4] as Map)
        .cast<String, Object?>();
    final references = badQuest['references']! as List;
    (references[1] as Map)['qualifier'] = '02';
    expect(
      () => Revision3ContentIndex.fromJsonObject(badSlot),
      throwsFormatException,
    );

    final foreignTarget = _contentIndexJson();
    final foreignQuest = ((foreignTarget['entities']! as List)[4] as Map)
        .cast<String, Object?>();
    final foreignReferences = foreignQuest['references']! as List;
    final foreignLineTarget = ((foreignReferences[1] as Map)['target']! as Map);
    foreignLineTarget['project_id'] = 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
    expect(
      () => Revision3ContentIndex.fromJsonObject(foreignTarget),
      throwsFormatException,
    );
  });

  test(
    'ContentIndex diagnoses truthful unresolved Quest refs while authoring rejects them',
    () async {
      for (final referenceIndex in <int>[0, 1]) {
        final unresolved = _contentIndexJson();
        final quest = ((unresolved['entities']! as List)[4] as Map)
            .cast<String, Object?>();
        final reference = (quest['references']! as List)[referenceIndex] as Map;
        final target = (reference['target']! as Map).cast<String, Object?>();
        target['entity_id'] = referenceIndex == 0
            ? '77777777777777777777777777777777'
            : '88888888888888888888888888888888';
        reference['resolution'] = 'missing_entity';

        final index = Revision3ContentIndex.fromJsonObject(unresolved);
        expect(index.entityById(_questId), isNotNull);
        await expectLater(
          _service(
            index: index,
            head: manifestHead(4096, 'b'),
          ).load(questId: _questId, expectedQuestRevision: 4),
          throwsA(isA<Revision3QuestTranscriptStaleCheckpointException>()),
        );
      }
    },
  );

  test(
    'projection preserves transcript order, stable slots and Voice coverage',
    () async {
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      final service = _service(index: index, head: head);

      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );

      expect(projection.checkpointIdentity, head.canonicalJson);
      expect(
        projection.objectives.map(
          (objective) => (objective.slot, objective.title),
        ),
        <(int, String)>[(1, 'Question Asghan'), (2, 'Inspect the gate')],
      );
      expect(projection.rows, hasLength(1));
      final row = projection.rows.single;
      expect(row.objectiveSlot, 2);
      expect(row.displayLabel, 'Dialog line');
      expect(row.speakerLabel, isNull);
      expect(row.authoredLocales, <String>['de']);
      expect(row.voiceSlotCount, 1);
      expect(row.voiceTakeCount, 1);
      expect(row.selectedVoiceTakeCount, 1);
      expect(
        row.localeCoverage.single.targetResolution,
        Revision3ContentVoiceTargetResolution.resolved,
      );
      expect(row.displayLabel, isNot(contains(_lineId)));
      expect(row.localizationStableKey, hasLength(24));
    },
  );

  test(
    'draft supports detach, attach, grouping and exact replace publication',
    () async {
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      Revision3QuestTranscriptReplaceTechnicalPlan? captured;
      final service = _service(
        index: index,
        head: head,
        replace:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required expectedHead,
              required plan,
            }) async {
              captured = plan;
              return const Revision3QuestTranscriptPublication(
                projectId: _projectId,
                projectRevision: 8,
                questId: _questId,
                questRevision: 5,
                moduleId: _moduleId,
                moduleRevision: 5,
                mode: AuthoringRevision3QuestTranscriptMode.replace,
                transcriptCount: 1,
                createdLineId: null,
                createdLocalizationId: null,
                createdVoiceSlotId: null,
                localizationAction: null,
              );
            },
      );
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection);
      final detached = draft.detachAt(0);
      expect(draft.unboundChoices, <Revision3QuestTranscriptLineChoice>[
        detached.line,
      ]);
      draft.attach(detached.line, objectiveSlot: 1, index: 0);
      draft.setObjectiveSlot(index: 0, objectiveSlot: 2);
      draft.setObjectiveSlot(index: 0, objectiveSlot: 1);

      final published = await service.replace(
        projection: projection,
        draft: draft,
      );

      expect(published.projectRevision, 8);
      expect(captured, isNotNull);
      expect(captured!.expectedModuleId, _moduleId);
      expect(captured!.expectedModuleRevision, 5);
      expect(captured!.bindings.single.lineId, _lineId);
      expect(captured!.bindings.single.objectiveSlot, 1);
    },
  );

  test('text preview is lazy and exact-head bound', () async {
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    var reads = 0;
    final service = _service(
      index: index,
      head: head,
      read:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            reads++;
            final request = AuthoringRevision3DialogLocalizationReadRequestV1(
              expectedHead: expectedHead,
              localizationId: localizationId,
              expectedLocalizationRevision: expectedLocalizationRevision,
              expectedLocId: expectedLocId,
            );
            return AuthoringRevision3DialogLocalizationReadResult.fromJson(
              <String, Object?>{
                'ok': true,
                'outcome': 'read_only',
                'head_json': expectedHead.canonicalJson,
                'project_id': expectedProjectId,
                'project_revision': expectedProjectRevision,
                'localization_id': localizationId,
                'localization_revision': expectedLocalizationRevision,
                'loc_id': expectedLocId,
                'locales': <Object?>[
                  <String, Object?>{
                    'locale': 'de',
                    'preview': 'Das Tor ist gesichert.',
                    'truncated': false,
                    'has_nonempty_text': true,
                  },
                ],
                'content_authority': 'read_only_exact_current_localization',
                'build_status': 'not_evaluated',
                'runtime_status': 'runtime_unqualified',
                'publication_status': 'not_applicable',
              },
              request: request,
            );
          },
    );
    final projection = await service.load(
      questId: _questId,
      expectedQuestRevision: 4,
    );
    expect(reads, 0);

    final preview = await service.loadTextPreview(
      projection: projection,
      row: projection.rows.single,
    );

    expect(reads, 1);
    expect(preview.displayLabel, 'Dialog line');
    expect(preview.locales.single.locale, 'de');
    expect(preview.locales.single.text, 'Das Tor ist gesichert.');
    expect(preview.locales.single.hasNonemptyText, isTrue);
  });

  test('foreign row object is rejected before exact text read', () async {
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    var reads = 0;
    final service = _service(
      index: index,
      head: head,
      read:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            reads++;
            throw StateError('must not read');
          },
    );
    final projection = await service.load(
      questId: _questId,
      expectedQuestRevision: 4,
    );
    final foreign = Revision3QuestTranscriptRow(
      line: projection.rows.single.line,
      objectiveSlot: 2,
    );

    await expectLater(
      service.loadTextPreview(projection: projection, row: foreign),
      throwsA(isA<Revision3QuestTranscriptStaleCheckpointException>()),
    );
    expect(reads, 0);
  });

  test(
    'managed session publishes one transcript CAS and fully reopens it',
    () async {
      final root = await Directory.systemTemp.createTemp('quest-transcript-');
      final store = _TranscriptSessionStore();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _managedProjectJson(),
      );
      addTearDown(() async {
        await session.close();
        if (await root.exists()) await root.delete(recursive: true);
      });
      store.openCalls = 0;
      store.openHeadCalls = 0;
      final basisHead = session.head.canonicalJson;
      final service = _sessionService(session, store);
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
        ..setObjectiveSlot(index: 0, objectiveSlot: 1);

      final publication = await service.replace(
        projection: projection,
        draft: draft,
      );

      expect(publication.projectRevision, 8);
      expect(session.projectRevision, 8);
      expect(session.head.canonicalJson, isNot(basisHead));
      expect(session.requiresReopen, isFalse);
      expect(store.transcriptCalls, 1);
      expect(store.openHeadCalls, greaterThanOrEqualTo(1));
      expect(store.openCalls, 1);
      expect(
        await File(p.join(root.path, 'gore-project.json')).readAsString(),
        session.head.canonicalJson,
      );
    },
  );

  test(
    'managed session latches reopen after uncertain published reopen',
    () async {
      final root = await Directory.systemTemp.createTemp('quest-transcript-');
      final store = _TranscriptSessionStore(failPublishedOpen: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _managedProjectJson(),
      );
      addTearDown(() async {
        await session.close();
        if (await root.exists()) await root.delete(recursive: true);
      });
      final service = _sessionService(session, store);
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
        ..setObjectiveSlot(index: 0, objectiveSlot: 1);

      await expectLater(
        service.replace(projection: projection, draft: draft),
        throwsStateError,
      );
      expect(session.requiresReopen, isTrue);
    },
  );

  test(
    'current-project bridge publishes only the selected exact checkpoint',
    () async {
      final lease = _TranscriptControllerLease(
        root: Directory('controller-transcript'),
        head: manifestHead(4096, 'b'),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);
      final service = _controllerService(
        coordinator: coordinator,
        visible: visible,
        index: Revision3ContentIndex.fromJsonObject(_contentIndexJson()),
      );
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
        ..setObjectiveSlot(index: 0, objectiveSlot: 1);

      final publication = await service.replace(
        projection: projection,
        draft: draft,
      );

      expect(publication.projectRevision, 8);
      expect(lease.replaceCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
    },
  );

  test(
    'current-project bridge rejects a project switch before publication',
    () async {
      final first = _TranscriptControllerLease(
        root: Directory('controller-transcript-first'),
        head: manifestHead(4096, 'b'),
      );
      final second = _TranscriptControllerLease(
        root: Directory('controller-transcript-second'),
        head: manifestHead(5000, 'c'),
      );
      var next = first;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => next,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(first.root);
      final service = _controllerService(
        coordinator: coordinator,
        visible: visible,
        index: Revision3ContentIndex.fromJsonObject(_contentIndexJson()),
      );
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
        ..setObjectiveSlot(index: 0, objectiveSlot: 1);
      next = second;
      await coordinator.openManagedRevision3(second.root);

      await expectLater(
        service.replace(projection: projection, draft: draft),
        throwsA(isA<Revision3QuestTranscriptStaleCheckpointException>()),
      );
      expect(first.replaceCalls, 0);
      expect(second.replaceCalls, 0);
    },
  );

  test(
    'current-project bridge poisons an unverifiable transcript receipt',
    () async {
      final lease = _TranscriptControllerLease(
        root: Directory('controller-transcript-uncertain'),
        head: manifestHead(4096, 'b'),
        mismatchReceipt: true,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);
      final service = _controllerService(
        coordinator: coordinator,
        visible: visible,
        index: Revision3ContentIndex.fromJsonObject(_contentIndexJson()),
      );
      final projection = await service.load(
        questId: _questId,
        expectedQuestRevision: 4,
      );
      final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
        ..setObjectiveSlot(index: 0, objectiveSlot: 1);

      await expectLater(
        service.replace(projection: projection, draft: draft),
        throwsA(isA<Revision3QuestTranscriptRequiresReopenException>()),
      );
      expect(lease.requiresReopen, isTrue);
      expect(lease.uncertaintyLatchCalls, 1);
    },
  );

  test('current-project bridge keeps binding rejection retryable', () async {
    final lease = _TranscriptControllerLease(
      root: Directory('controller-transcript-rejected'),
      head: manifestHead(4096, 'b'),
      rejectBinding: true,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => lease,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(lease.root);
    final service = _controllerService(
      coordinator: coordinator,
      visible: visible,
      index: Revision3ContentIndex.fromJsonObject(_contentIndexJson()),
    );
    final projection = await service.load(
      questId: _questId,
      expectedQuestRevision: 4,
    );
    final draft = Revision3QuestTranscriptDraft.fromProjection(projection)
      ..setObjectiveSlot(index: 0, objectiveSlot: 1);

    await expectLater(
      service.replace(projection: projection, draft: draft),
      throwsA(isA<Revision3QuestTranscriptStaleCheckpointException>()),
    );
    expect(lease.requiresReopen, isFalse);
    expect(lease.uncertaintyLatchCalls, 0);
  });
}

Revision3QuestTranscriptAuthoringService _service({
  required Revision3ContentIndex index,
  required AuthoringWorkingHead head,
  Revision3QuestTranscriptLocalizationReader? read,
  Revision3QuestTranscriptReplacePublisher? replace,
}) => Revision3QuestTranscriptAuthoringService(
  expectedHead: head,
  loadContentIndex: () async => index,
  readExactLocalization:
      read ??
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required localizationId,
        required expectedLocalizationRevision,
        required expectedLocId,
      }) async => throw StateError('unexpected preview read'),
  publishReplace:
      replace ??
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected replace'),
  publishCreate:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected create'),
);

Revision3QuestTranscriptAuthoringService _sessionService(
  ManagedRevision3AuthoringProjectSession session,
  _TranscriptSessionStore store,
) => Revision3QuestTranscriptAuthoringService(
  expectedHead: session.head,
  loadContentIndex: () async =>
      Revision3ContentIndex.fromJsonObject(_contentIndexJson()),
  readExactLocalization:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required localizationId,
        required expectedLocalizationRevision,
        required expectedLocId,
      }) async => throw StateError('unexpected preview read'),
  publishReplace:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async {
        final checkpoint = await session
            .prepareAndPublishQuestTranscriptReplaceV1(plan: plan);
        return Revision3QuestTranscriptPublication(
          projectId: checkpoint.projectId,
          projectRevision: checkpoint.projectRevision,
          questId: checkpoint.questId,
          questRevision: checkpoint.questRevision,
          moduleId: checkpoint.moduleId,
          moduleRevision: checkpoint.moduleRevision,
          mode: checkpoint.mode,
          transcriptCount: checkpoint.transcriptCount,
          createdLineId: checkpoint.createdLineId,
          createdLocalizationId: checkpoint.createdLocalizationId,
          createdVoiceSlotId: checkpoint.createdVoiceSlotId,
          localizationAction: checkpoint.localizationAction,
        );
      },
  publishCreate:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected create'),
);

Revision3QuestTranscriptAuthoringService _controllerService({
  required CurrentProjectCoordinator coordinator,
  required ManagedRevision3CurrentProjectState visible,
  required Revision3ContentIndex index,
}) => Revision3QuestTranscriptAuthoringService(
  expectedHead: visible.head,
  loadContentIndex: () async => index,
  readExactLocalization:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required localizationId,
        required expectedLocalizationRevision,
        required expectedLocId,
      }) async => throw StateError('unexpected preview read'),
  publishReplace:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) => coordinator.replaceCurrentRevision3QuestTranscript(
        expectedRoot: visible.root.path,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        expectedHead: expectedHead,
        plan: plan,
      ),
  publishCreate:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) => coordinator.createCurrentRevision3QuestTranscriptLine(
        expectedRoot: visible.root.path,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        expectedHead: expectedHead,
        plan: plan,
      ),
);

final class _TranscriptControllerLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3QuestTranscriptLease {
  _TranscriptControllerLease({
    required this.root,
    required this.head,
    this.mismatchReceipt = false,
    this.rejectBinding = false,
  });

  @override
  final Directory root;
  @override
  AuthoringWorkingHead head;
  final bool mismatchReceipt;
  final bool rejectBinding;
  @override
  String get projectId => _projectId;
  @override
  int projectRevision = 7;
  @override
  String canonicalProjectJson = '{}';
  bool _requiresReopen = false;
  int replaceCalls = 0;
  int uncertaintyLatchCalls = 0;
  int closeCalls = 0;

  @override
  bool get requiresReopen => _requiresReopen;

  @override
  bool get supportsQuestTranscript => true;

  @override
  void markRequiresReopenAfterQuestTranscriptUncertainty() {
    uncertaintyLatchCalls++;
    _requiresReopen = true;
  }

  @override
  Future<Revision3QuestTranscriptPublication>
  prepareAndPublishQuestTranscriptReplaceV1({
    required Revision3QuestTranscriptReplaceTechnicalPlan plan,
  }) async {
    replaceCalls++;
    if (rejectBinding) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transcript_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSCRIPT_BINDING_CONFLICT',
        message: 'fake semantic binding rejection',
      );
    }
    projectRevision++;
    head = manifestHead(5000 + replaceCalls, 'd');
    return Revision3QuestTranscriptPublication(
      projectId: _projectId,
      projectRevision: projectRevision,
      questId: plan.questId,
      questRevision: plan.expectedQuestRevision + 1,
      moduleId: plan.expectedModuleId,
      moduleRevision: plan.expectedModuleRevision,
      mode: AuthoringRevision3QuestTranscriptMode.replace,
      transcriptCount: plan.bindings.length + (mismatchReceipt ? 1 : 0),
      createdLineId: null,
      createdLocalizationId: null,
      createdVoiceSlotId: null,
      localizationAction: null,
    );
  }

  @override
  Future<Revision3QuestTranscriptPublication>
  prepareAndPublishQuestTranscriptCreateV1({
    required Revision3QuestTranscriptCreateTechnicalPlan plan,
  }) => throw StateError('unexpected create');

  @override
  Future<void> close() async {
    closeCalls++;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

final class _TranscriptSessionStore
    implements
        ManagedRevision3AuthoringStore,
        ManagedRevision3QuestTranscriptStore {
  _TranscriptSessionStore({this.failPublishedOpen = false});

  final bool failPublishedOpen;
  final Map<String, String> _projects = <String, String>{};
  int _sequence = 0;
  bool _failNextOpen = false;
  int openCalls = 0;
  int openHeadCalls = 0;
  int transcriptCalls = 0;

  AuthoringWorkingHead _register(String projectJson) {
    _sequence++;
    final head = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': _sequence.toRadixString(16).padLeft(64, '0'),
        },
      }),
    );
    _projects[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async =>
      AuthoringRevision3CheckpointPreparation.fromJson(<String, Object?>{
        'ok': true,
        'head_json': _register(projectJson).canonicalJson,
      });

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    openHeadCalls++;
    final project = _projects[head.canonicalJson];
    if (project == null) throw StateError('unknown fake checkpoint');
    return AuthoringRevision3StoreOpenedResult.fromJson(<String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
      'project_json': project,
    });
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    openCalls++;
    if (_failNextOpen) {
      _failNextOpen = false;
      throw StateError('uncertain published reopen');
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      await File(p.join(root, 'gore-project.json')).readAsString(),
    );
    final project = _projects[head.canonicalJson];
    if (project == null) throw StateError('unknown fake published head');
    return AuthoringRevision3StoreOpenedResult.fromJson(<String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
      'project_json': project,
    });
  }

  @override
  Future<AuthoringRevision3QuestTranscriptPreparation>
  prepareQuestTranscriptV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTranscriptRequestV1 request,
  }) async {
    transcriptCalls++;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projects[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transcript_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT',
        message: 'fake transcript basis drifted',
      );
    }
    final intent = request.intent;
    if (intent is! AuthoringRevision3QuestTranscriptReplaceIntentV1) {
      throw StateError('fake supports replace only');
    }
    final candidate = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    candidate['revision'] = request.expectedRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final quest = (entities[request.questId]! as Map).cast<String, Object?>();
    quest['revision'] = request.expectedQuestRevision + 1;
    final payload = (quest['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    if (intent.bindings.isEmpty) {
      data.remove('transcript');
    } else {
      data['transcript'] = <Object?>[
        for (final binding in intent.bindings)
          <String, Object?>{
            'line': <String, Object?>{
              'project_id': binding.projectId,
              'id': binding.lineId,
              'expected_kind': 'dialog_line',
            },
            'objective_slot': binding.objectiveSlot,
          },
      ];
    }
    final candidateJson = jsonEncode(candidate);
    final candidateHead = _register(candidateJson);
    if (failPublishedOpen) _failNextOpen = true;
    return AuthoringRevision3QuestTranscriptPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': request.expectedHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateJson,
        'project_id': request.expectedProjectId,
        'revision': request.expectedRevision + 1,
        'quest_id': request.questId,
        'quest_revision': request.expectedQuestRevision + 1,
        'module_id': request.moduleId,
        'module_revision': request.expectedModuleRevision,
        'mode': 'replace',
        'transcript_count': intent.bindings.length,
        'created_line_id': null,
        'created_localization_id': null,
        'created_voice_slot_id': null,
        'localization_action': null,
        'build_status': 'blocked',
        'runtime_status': 'runtime_unqualified',
        'topic_authority': 'not_granted',
        'publication_status': 'not_supported',
      },
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

String _managedProjectJson() => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': 7,
  'meta': <String, Object?>{
    'name': 'Transcript fixture',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 99, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de'],
  'entities': <String, Object?>{
    _localizationId: <String, Object?>{
      'id': _localizationId,
      'display_name': 'Gate warning text',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'DIA_GATE_WARNING',
      },
      'revision': 3,
      'payload': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': 'DIA_GATE_WARNING',
          'texts': <String, Object?>{'de': 'Das Tor ist gesichert.'},
        },
      },
    },
    _lineId: <String, Object?>{
      'id': _lineId,
      'display_name': 'Gate warning',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'DIA_GATE_WARNING_LINE',
      },
      'revision': 2,
      'payload': <String, Object?>{
        'kind': 'dialog_line',
        'data': <String, Object?>{
          'localization': <String, Object?>{
            'project_id': _projectId,
            'id': _localizationId,
            'expected_kind': 'localization_entry',
          },
          'speaker_hint': 'Asghan',
          'voice_slots': <String, Object?>{},
        },
      },
    },
    _questId: <String, Object?>{
      'id': _questId,
      'display_name': 'Secure the gate',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_SECURE_GATE',
      },
      'revision': 4,
      'payload': <String, Object?>{
        'kind': 'quest_draft',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'input': <String, Object?>{
            'transition_plan':
                AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
                  2,
                ).toJson(),
          },
          'script_module': <String, Object?>{
            'project_id': _projectId,
            'id': _moduleId,
            'expected_kind': 'script_module',
          },
          'transcript': <Object?>[
            <String, Object?>{
              'line': <String, Object?>{
                'project_id': _projectId,
                'id': _lineId,
                'expected_kind': 'dialog_line',
              },
              'objective_slot': 2,
            },
          ],
        },
      },
    },
    _moduleId: <String, Object?>{
      'id': _moduleId,
      'display_name': 'Secure the gate script',
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': _projectId,
          'id': _questId,
          'expected_kind': 'quest_draft',
        },
      },
      'revision': 5,
      'payload': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{},
      },
    },
  },
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _contentIndexJson() => <String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': 7,
  'project_name': 'Transcript fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 99, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de'],
  'entity_counts': <String, Object?>{
    'localization_entry': 1,
    'dialog_line': 1,
    'voice_slot': 1,
    'voice_take': 1,
    'quest_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    _entity(
      id: _localizationId,
      kind: 'localization_entry',
      displayName: 'Gate warning text',
      revision: 3,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_GATE_WARNING',
        'locales': <Object?>['de'],
      },
    ),
    _entity(
      id: _lineId,
      kind: 'dialog_line',
      displayName: _lineId,
      revision: 2,
      summaryData: <String, Object?>{
        'speaker_hint': _lineId,
        'voice_slot_locales': <Object?>['de'],
      },
      references: <Object?>[
        _reference(
          role: 'dialog_localization',
          targetId: _localizationId,
          expectedKind: 'localization_entry',
        ),
        _reference(
          role: 'dialog_voice_slot',
          qualifier: 'de',
          targetId: _slotId,
          expectedKind: 'voice_slot',
        ),
      ],
    ),
    _entity(
      id: _slotId,
      kind: 'voice_slot',
      displayName: 'German Voice',
      revision: 1,
      origin: _generatedOrigin(
        ownerId: _lineId,
        ownerKind: 'dialog_line',
        generatorId: 'gore-authoring.dialog-voice-slot',
      ),
      summaryData: <String, Object?>{
        'locale': 'de',
        'target_resolution': 'resolved',
        'candidate_count': 1,
        'has_selected_take': true,
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _lineId,
          expectedKind: 'dialog_line',
        ),
        _reference(
          role: 'voice_candidate',
          targetId: _takeId,
          expectedKind: 'voice_take',
        ),
        _reference(
          role: 'voice_selected',
          targetId: _takeId,
          expectedKind: 'voice_take',
        ),
      ],
    ),
    _entity(
      id: _takeId,
      kind: 'voice_take',
      displayName: 'Take 1',
      revision: 1,
      summaryData: <String, Object?>{
        'locale': 'de',
        'status': 'recorded',
        'codec': 'vorbis',
        'channels': 1,
        'sample_rate': 44100,
      },
    ),
    _entity(
      id: _questId,
      kind: 'quest_draft',
      displayName: 'Secure the gate',
      revision: 4,
      summaryData: <String, Object?>{
        'technical_id': 'GORE_SECURE_GATE',
        'title': 'Secure the gate',
        'objective_title': 'Question Asghan',
        'additional_objective_titles': <Object?>['Inspect the gate'],
        'objective_slots': <Object?>[1, 2],
        'transcript_count': 1,
        'module_namespace': 'PROJECT.QUESTS.SECUREGATE',
        'parent_runtime_class': 'UQuest_SwampCamp',
        'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
      },
      references: <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
        _reference(
          role: 'quest_transcript_line',
          qualifier: '2',
          targetId: _lineId,
          expectedKind: 'dialog_line',
        ),
      ],
    ),
    _entity(
      id: _moduleId,
      kind: 'script_module',
      displayName: 'Secure the gate script',
      revision: 5,
      origin: _generatedOrigin(
        ownerId: _questId,
        ownerKind: 'quest_draft',
        generatorId: 'gore-authoring.draft-quest-skeleton',
      ),
      summaryData: <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'module_namespace': 'PROJECT.QUESTS.SECUREGATE',
        'module_relative_path': 'PROJECT/QUESTS/SECUREGATE.as',
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _questId,
          expectedKind: 'quest_draft',
        ),
      ],
    ),
  ],
  'assets': <Object?>[],
};

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summaryData,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
}) => <String, Object?>{
  'id': id,
  'kind': kind,
  'display_name': displayName,
  'revision': revision,
  'origin':
      origin ??
      <String, Object?>{'type': 'new', 'authored_runtime_id': 'AUTHORED_$kind'},
  'summary': <String, Object?>{'kind': kind, 'data': summaryData},
  'references': references,
  'asset_references': <Object?>[],
};

Map<String, Object?> _generatedOrigin({
  required String ownerId,
  required String ownerKind,
  required String generatorId,
}) => <String, Object?>{
  'type': 'generated',
  'generator_id': generatorId,
  'generator_version': 1,
  'owner': <String, Object?>{
    'project_id': _projectId,
    'entity_id': ownerId,
    'expected_kind': ownerKind,
  },
};

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': _projectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
