import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_npc_greeting_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_npc_fixture.dart';
import '../support/revision3_npc_profile_edit_fixture.dart';

const _projectId = revision3NpcProfileProjectId;
const _npcId = revision3NpcProfileNpcId;
const _moduleId = revision3NpcProfileModuleId;
const _localizationA = '11111111111111111111111111111111';
const _lineA = '22222222222222222222222222222222';
const _localizationB = '33333333333333333333333333333333';
const _lineB = '44444444444444444444444444444444';
const _targetSha = revision3NpcProfileExecutableSha256;

void main() {
  test('ContentIndex validates ordered NPC greeting facts and backlinks', () {
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    final npc = index.entityById(_npcId)!;
    expect(npc.summary.npcDraft!.greetingCount, 2);
    expect(
      npc.references
          .where((reference) => reference.role == 'npc_greeting_line')
          .map((reference) => reference.target.entityId),
      <String>[_lineB, _lineA],
    );
    expect(
      index
          .backlinksToEntity(_lineB)
          .where((backlink) => backlink.reference.role == 'npc_greeting_line')
          .single
          .source
          .id,
      _npcId,
    );

    final falseCount = _contentIndexJson();
    final falseNpc = ((falseCount['entities']! as List)[0] as Map)
        .cast<String, Object?>();
    final summary = (falseNpc['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['greeting_count'] = 1;
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseCount),
      throwsFormatException,
    );

    final qualified = _contentIndexJson();
    final qualifiedNpc = ((qualified['entities']! as List)[0] as Map)
        .cast<String, Object?>();
    final references = qualifiedNpc['references']! as List;
    (references[1] as Map)['qualifier'] = 'first';
    expect(
      () => Revision3ContentIndex.fromJsonObject(qualified),
      throwsFormatException,
    );
  });

  test('ContentIndex requires the current NPC greeting projection', () {
    final missingCount = _contentIndexJson();
    final npc = ((missingCount['entities']! as List)[0] as Map)
        .cast<String, Object?>();
    final summary = (npc['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data.remove('greeting_count');

    expect(
      () => Revision3ContentIndex.fromJsonObject(missingCount),
      throwsFormatException,
    );
  });

  test(
    'projection never exposes NPC or generated-module technical identity',
    () async {
      final raw = _contentIndexJson();
      final line = ((raw['entities']! as List)[5] as Map)
          .cast<String, Object?>();
      line['display_name'] = 'GoreMods/Npcs/ManagedGuard.as';
      final summary = (line['summary']! as Map).cast<String, Object?>();
      final data = (summary['data']! as Map).cast<String, Object?>();
      data['speaker_hint'] = 'GoreManagedGuard';
      final service = _service(
        index: Revision3ContentIndex.fromJsonObject(raw),
        head: _head(4096, 'b'),
      );

      final projection = await service.load(
        npcId: _npcId,
        expectedNpcRevision: 4,
      );
      final row = projection.rows.first;

      expect(row.lineId, _lineB);
      expect(row.displayLabel, 'Dialog line');
      expect(row.speakerLabel, isNull);
      expect(row.displayLabel, isNot(contains('ManagedGuard')));
      expect(row.displayLabel, isNot(contains(_npcId)));
      expect(row.displayLabel, isNot(contains(_moduleId)));
    },
  );

  test(
    'projection, draft ordering and exact replacement stay checkpoint-bound',
    () async {
      final head = _head(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      Revision3NpcGreetingReplaceTechnicalPlan? captured;
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
              return const Revision3NpcGreetingPublication(
                projectId: _projectId,
                projectRevision: 8,
                npcId: _npcId,
                npcRevision: 5,
                moduleId: _moduleId,
                moduleRevision: 5,
                mode: AuthoringRevision3NpcGreetingMode.replace,
                greetingCount: 2,
                createdLineId: null,
                createdLocalizationId: null,
                createdVoiceSlotId: null,
                localizationAction: null,
              );
            },
      );
      final projection = await service.load(
        npcId: _npcId,
        expectedNpcRevision: 4,
      );

      expect(projection.checkpointIdentity, head.canonicalJson);
      expect(projection.rows.map((row) => row.lineId), <String>[
        _lineB,
        _lineA,
      ]);
      expect(projection.rows.map((row) => row.displayLabel), <String>[
        'Second greeting',
        'First greeting',
      ]);
      expect(projection.rows.first.authoredLocales, <String>['de']);
      expect(projection.rows.first.localizationStableKey, hasLength(24));
      expect(projection.rows.first.displayLabel, isNot(contains(_lineB)));

      final draft = Revision3NpcGreetingDraft.fromProjection(projection);
      final detached = draft.detachAt(0);
      expect(draft.unboundChoices, contains(detached.line));
      draft.attach(detached.line, index: 1);
      expect(draft.rows.map((row) => row.line.lineId), <String>[
        _lineA,
        _lineB,
      ]);

      final published = await service.replace(
        projection: projection,
        draft: draft,
      );
      expect(published.projectRevision, 8);
      expect(captured!.expectedModuleId, _moduleId);
      expect(captured!.expectedModuleRevision, 5);
      expect(captured!.bindings.map((binding) => binding.lineId), <String>[
        _lineA,
        _lineB,
      ]);
    },
  );

  test('text preview is lazy and exact-head bound', () async {
    final head = _head(4096, 'b');
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
                    'preview': 'Willkommen im Lager.',
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
      npcId: _npcId,
      expectedNpcRevision: 4,
    );
    expect(reads, 0);
    final preview = await service.loadTextPreview(
      projection: projection,
      row: projection.rows.first,
    );
    expect(reads, 1);
    expect(preview.locales.single.text, 'Willkommen im Lager.');
  });

  test(
    'create-and-insert carries one fresh DialogLine plan atomically',
    () async {
      final head = _head(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      final line = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
        catalog: Revision3DialogLineEntryCatalog.fromContentIndex(index),
        input: Revision3DialogLineEntryInput.create(
          lineDisplayName: 'Fresh greeting',
          speakerHint: 'Asghan',
          locale: 'de',
          text: 'Halt! Wer da?',
          createVoiceSlot: false,
        ),
      );
      Revision3NpcGreetingCreateTechnicalPlan? captured;
      final service = _service(
        index: index,
        head: head,
        create:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required expectedHead,
              required plan,
            }) async {
              captured = plan;
              return Revision3NpcGreetingPublication(
                projectId: _projectId,
                projectRevision: 8,
                npcId: _npcId,
                npcRevision: 5,
                moduleId: _moduleId,
                moduleRevision: 5,
                mode: AuthoringRevision3NpcGreetingMode.createAndInsert,
                greetingCount: 3,
                createdLineId: plan.line.lineId,
                createdLocalizationId: plan.line.localization.localizationId,
                createdVoiceSlotId: plan.line.voiceSlot?.slotId,
                localizationAction:
                    AuthoringRevision3DialogLocalizationAction.created,
              );
            },
      );
      final projection = await service.load(
        npcId: _npcId,
        expectedNpcRevision: 4,
      );

      final publication = await service.createAndInsert(
        projection: projection,
        index: 1,
        line: line,
      );

      expect(publication.createdLineId, line.lineId);
      expect(captured!.expectedGreetingCount, 2);
      expect(captured!.index, 1);
      expect(captured!.expectedModuleId, _moduleId);
    },
  );

  test(
    'managed session publishes one greeting CAS and fully reopens it',
    () async {
      final root = await Directory.systemTemp.createTemp('npc-greeting-');
      final store = _GreetingSessionStore();
      final projectJson = _managedProjectJson();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(() async {
        await session.close();
        if (await root.exists()) await root.delete(recursive: true);
      });
      store.openCalls = 0;
      store.openHeadCalls = 0;
      final basisHead = session.head.canonicalJson;
      final service = _sessionService(session);
      final projection = await service.load(
        npcId: _npcId,
        expectedNpcRevision: 0,
      );
      final draft = Revision3NpcGreetingDraft.fromProjection(projection)
        ..reorder(fromIndex: 0, toIndex: 1);

      final publication = await service.replace(
        projection: projection,
        draft: draft,
      );

      expect(publication.projectRevision, 2);
      expect(session.projectRevision, 2);
      expect(session.head.canonicalJson, isNot(basisHead));
      expect(session.requiresReopen, isFalse);
      expect(store.greetingCalls, 1);
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
      final root = await Directory.systemTemp.createTemp('npc-greeting-');
      final store = _GreetingSessionStore(failPublishedOpen: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _managedProjectJson(),
      );
      addTearDown(() async {
        await session.close();
        if (await root.exists()) await root.delete(recursive: true);
      });
      final service = _sessionService(session);
      final projection = await service.load(
        npcId: _npcId,
        expectedNpcRevision: 0,
      );
      final draft = Revision3NpcGreetingDraft.fromProjection(projection)
        ..reorder(fromIndex: 0, toIndex: 1);

      await expectLater(
        service.replace(projection: projection, draft: draft),
        throwsStateError,
      );
      expect(session.requiresReopen, isTrue);
    },
  );

  test(
    'current-project bridge poisons an unverifiable greeting receipt',
    () async {
      final lease = _GreetingControllerLease(
        root: Directory('controller-npc-greeting'),
        head: _head(4096, 'b'),
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
        npcId: _npcId,
        expectedNpcRevision: 4,
      );
      final draft = Revision3NpcGreetingDraft.fromProjection(projection)
        ..reorder(fromIndex: 0, toIndex: 1);

      await expectLater(
        service.replace(projection: projection, draft: draft),
        throwsA(isA<Revision3NpcGreetingRequiresReopenException>()),
      );
      expect(lease.requiresReopen, isTrue);
      expect(lease.uncertaintyLatchCalls, 1);
    },
  );
}

Revision3NpcGreetingAuthoringService _service({
  required Revision3ContentIndex index,
  required AuthoringWorkingHead head,
  Revision3NpcGreetingLocalizationReader? read,
  Revision3NpcGreetingReplacePublisher? replace,
  Revision3NpcGreetingCreatePublisher? create,
}) => Revision3NpcGreetingAuthoringService(
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
      create ??
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected create'),
);

Revision3NpcGreetingAuthoringService _sessionService(
  ManagedRevision3AuthoringProjectSession session,
) => Revision3NpcGreetingAuthoringService(
  expectedHead: session.head,
  loadContentIndex: () async => Revision3ContentIndex.fromJsonObject(
    _contentIndexJson(projectRevision: 1, npcRevision: 0, moduleRevision: 0),
  ),
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
        final checkpoint = await session.prepareAndPublishNpcGreetingReplaceV1(
          plan: plan,
        );
        return Revision3NpcGreetingPublication(
          projectId: checkpoint.projectId,
          projectRevision: checkpoint.projectRevision,
          npcId: checkpoint.npcId,
          npcRevision: checkpoint.npcRevision,
          moduleId: checkpoint.moduleId,
          moduleRevision: checkpoint.moduleRevision,
          mode: checkpoint.mode,
          greetingCount: checkpoint.greetingCount,
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

Revision3NpcGreetingAuthoringService _controllerService({
  required CurrentProjectCoordinator coordinator,
  required ManagedRevision3CurrentProjectState visible,
  required Revision3ContentIndex index,
}) => Revision3NpcGreetingAuthoringService(
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
      }) => coordinator.replaceCurrentRevision3NpcGreeting(
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
      }) => coordinator.createCurrentRevision3NpcGreetingLine(
        expectedRoot: visible.root.path,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        expectedHead: expectedHead,
        plan: plan,
      ),
);

final class _GreetingControllerLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3NpcGreetingLease {
  _GreetingControllerLease({required this.root, required this.head});

  @override
  final Directory root;
  @override
  AuthoringWorkingHead head;
  @override
  String get projectId => _projectId;
  @override
  int projectRevision = 7;
  @override
  String canonicalProjectJson = '{}';
  bool _requiresReopen = false;
  int uncertaintyLatchCalls = 0;

  @override
  bool get requiresReopen => _requiresReopen;
  @override
  bool get supportsNpcGreeting => true;

  @override
  void markRequiresReopenAfterNpcGreetingUncertainty() {
    uncertaintyLatchCalls++;
    _requiresReopen = true;
  }

  @override
  Future<Revision3NpcGreetingPublication>
  prepareAndPublishNpcGreetingReplaceV1({
    required Revision3NpcGreetingReplaceTechnicalPlan plan,
  }) async {
    projectRevision++;
    head = _head(5000, 'd');
    return Revision3NpcGreetingPublication(
      projectId: _projectId,
      projectRevision: projectRevision,
      npcId: plan.npcId,
      npcRevision: plan.expectedNpcRevision + 1,
      moduleId: plan.expectedModuleId,
      moduleRevision: plan.expectedModuleRevision,
      mode: AuthoringRevision3NpcGreetingMode.replace,
      greetingCount: plan.bindings.length + 1,
      createdLineId: null,
      createdLocalizationId: null,
      createdVoiceSlotId: null,
      localizationAction: null,
    );
  }

  @override
  Future<Revision3NpcGreetingPublication> prepareAndPublishNpcGreetingCreateV1({
    required Revision3NpcGreetingCreateTechnicalPlan plan,
  }) => throw StateError('unexpected create');

  @override
  Future<void> close() async {}

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

final class _GreetingSessionStore
    implements
        ManagedRevision3AuthoringStore,
        ManagedRevision3NpcGreetingStore {
  _GreetingSessionStore({this.failPublishedOpen = false});

  final bool failPublishedOpen;
  final Map<String, String> _projects = <String, String>{};
  int _sequence = 0;
  bool _failNextOpen = false;
  int openCalls = 0;
  int openHeadCalls = 0;
  int greetingCalls = 0;

  AuthoringWorkingHead _register(String projectJson) {
    _sequence++;
    final head = _head(
      utf8.encode(projectJson).length,
      _sequence.toRadixString(16),
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
  Future<AuthoringRevision3NpcGreetingPreparation> prepareNpcGreetingV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3NpcGreetingRequestV1 request,
  }) async {
    greetingCalls++;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projects[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_npc_greeting_v1',
        code: 'AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT',
        message: 'fake greeting basis drifted',
      );
    }
    final intent = request.intent;
    if (intent is! AuthoringRevision3NpcGreetingReplaceIntentV1) {
      throw StateError('fake supports replace only');
    }
    final candidate = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    candidate['revision'] = request.expectedRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final npc = (entities[request.npcId]! as Map).cast<String, Object?>();
    npc['revision'] = request.expectedNpcRevision + 1;
    final payload = (npc['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    data['greetings'] = <Object?>[
      for (final binding in intent.bindings)
        <String, Object?>{
          'line': <String, Object?>{
            'project_id': binding.projectId,
            'id': binding.lineId,
            'expected_kind': 'dialog_line',
          },
        },
    ];
    final candidateJson = jsonEncode(candidate);
    final candidateHead = _register(candidateJson);
    if (failPublishedOpen) _failNextOpen = true;
    return AuthoringRevision3NpcGreetingPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': request.expectedHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateJson,
        'project_id': request.expectedProjectId,
        'revision': request.expectedRevision + 1,
        'npc_id': request.npcId,
        'npc_revision': request.expectedNpcRevision + 1,
        'module_id': request.moduleId,
        'module_revision': request.expectedModuleRevision,
        'mode': 'replace',
        'greeting_count': intent.bindings.length,
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

String _managedProjectJson() {
  final fixture = Revision3NpcProfileTestFixture.create();
  final project = (jsonDecode(fixture.projectJson) as Map)
      .cast<String, Object?>();
  project['authoring_locales'] = <Object?>['de'];
  final entities = (project['entities']! as Map).cast<String, Object?>();
  entities[_localizationA] = _projectLocalization(
    id: _localizationA,
    authoredId: 'DIA_FIRST_GREETING',
    text: 'Erste Begruessung.',
  );
  entities[_lineA] = _projectLine(
    id: _lineA,
    localizationId: _localizationA,
    authoredId: 'DIA_FIRST_GREETING_LINE',
  );
  entities[_localizationB] = _projectLocalization(
    id: _localizationB,
    authoredId: 'DIA_SECOND_GREETING',
    text: 'Zweite Begruessung.',
  );
  entities[_lineB] = _projectLine(
    id: _lineB,
    localizationId: _localizationB,
    authoredId: 'DIA_SECOND_GREETING_LINE',
  );
  final npc = (entities[_npcId]! as Map).cast<String, Object?>();
  final payload = (npc['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['greetings'] = <Object?>[
    _projectGreeting(_lineB),
    _projectGreeting(_lineA),
  ];
  final ids = entities.keys.toList()..sort();
  project['entities'] = <String, Object?>{
    for (final id in ids) id: entities[id],
  };
  return jsonEncode(project);
}

Map<String, Object?> _contentIndexJson({
  int projectRevision = 7,
  int npcRevision = 4,
  int moduleRevision = 5,
}) => <String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': projectRevision,
  'project_name': 'NPC greeting fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': revision3NpcProfileExecutableByteLength,
      'sha256': _targetSha,
    },
  },
  'authoring_locales': <Object?>['de'],
  'entity_counts': <String, Object?>{
    'localization_entry': 2,
    'dialog_line': 2,
    'npc_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    _entity(
      id: _npcId,
      kind: 'npc_draft',
      displayName: 'Managed Guard',
      revision: npcRevision,
      summaryData: <String, Object?>{
        'unique_name': 'GoreManagedGuard',
        'module_namespace': 'GoreMods.Npcs.ManagedGuard',
        'parent_character_definition': 'UCharacterDefinition_Asghan',
        'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
        'parent_spawn_definition': 'USpawnDefinition_Asghan',
        'greeting_count': 2,
      },
      references: <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
        _reference(
          role: 'npc_greeting_line',
          targetId: _lineB,
          expectedKind: 'dialog_line',
        ),
        _reference(
          role: 'npc_greeting_line',
          targetId: _lineA,
          expectedKind: 'dialog_line',
        ),
      ],
    ),
    _entity(
      id: _moduleId,
      kind: 'script_module',
      displayName: 'Managed Guard script',
      revision: moduleRevision,
      origin: _generatedOrigin(),
      summaryData: <String, Object?>{
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': 1,
        'module_namespace': 'GoreMods.Npcs.ManagedGuard',
        'module_relative_path': 'GoreMods/Npcs/ManagedGuard.as',
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _npcId,
          expectedKind: 'npc_draft',
        ),
      ],
    ),
    _entity(
      id: _localizationA,
      kind: 'localization_entry',
      displayName: 'First greeting text',
      revision: 3,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_FIRST_GREETING',
        'locales': <Object?>['de'],
      },
    ),
    _entity(
      id: _lineA,
      kind: 'dialog_line',
      displayName: 'First greeting',
      revision: 2,
      summaryData: <String, Object?>{
        'speaker_hint': 'Asghan',
        'voice_slot_locales': <Object?>[],
      },
      references: <Object?>[
        _reference(
          role: 'dialog_localization',
          targetId: _localizationA,
          expectedKind: 'localization_entry',
        ),
      ],
    ),
    _entity(
      id: _localizationB,
      kind: 'localization_entry',
      displayName: 'Second greeting text',
      revision: 4,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_SECOND_GREETING',
        'locales': <Object?>['de'],
      },
    ),
    _entity(
      id: _lineB,
      kind: 'dialog_line',
      displayName: 'Second greeting',
      revision: 3,
      summaryData: <String, Object?>{
        'speaker_hint': 'Asghan',
        'voice_slot_locales': <Object?>[],
      },
      references: <Object?>[
        _reference(
          role: 'dialog_localization',
          targetId: _localizationB,
          expectedKind: 'localization_entry',
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

Map<String, Object?> _generatedOrigin() => <String, Object?>{
  'type': 'generated',
  'generator_id': revision3NpcFixtureGeneratorId,
  'generator_version': 1,
  'owner': <String, Object?>{
    'project_id': _projectId,
    'entity_id': _npcId,
    'expected_kind': 'npc_draft',
  },
};

Map<String, Object?> _reference({
  required String role,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': _projectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Map<String, Object?> _projectGreeting(String lineId) => <String, Object?>{
  'line': <String, Object?>{
    'project_id': _projectId,
    'id': lineId,
    'expected_kind': 'dialog_line',
  },
};

Map<String, Object?> _projectLocalization({
  required String id,
  required String authoredId,
  required String text,
}) => <String, Object?>{
  'id': id,
  'display_name': authoredId,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': authoredId},
  'revision': 0,
  'payload': <String, Object?>{
    'kind': 'localization_entry',
    'data': <String, Object?>{
      'loc_id': authoredId,
      'texts': <String, Object?>{'de': text},
    },
  },
};

Map<String, Object?> _projectLine({
  required String id,
  required String localizationId,
  required String authoredId,
}) => <String, Object?>{
  'id': id,
  'display_name': authoredId,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': authoredId},
  'revision': 0,
  'payload': <String, Object?>{
    'kind': 'dialog_line',
    'data': <String, Object?>{
      'localization': <String, Object?>{
        'project_id': _projectId,
        'id': localizationId,
        'expected_kind': 'localization_entry',
      },
      'speaker_hint': 'Asghan',
      'voice_slots': <String, Object?>{},
    },
  },
};

AuthoringWorkingHead _head(int byteLength, String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': byteLength,
          'sha256': digit.padLeft(64, digit),
        },
      }),
    );
