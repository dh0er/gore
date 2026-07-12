import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/story/domain/story_draft_requests.dart';
import 'package:gore_mod/story/domain/story_workspace_controller.dart';
import 'package:path/path.dart' as p;

const _projectId = '01010101010101010101010101010101';
const _firstDraftId = '10101010101010101010101010101010';
const _firstModuleId = '11111111111111111111111111111111';
const _secondDraftId = '20202020202020202020202020202020';
const _secondModuleId = '21212121212121212121212121212121';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp('gore_story_controller_');
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test(
    'two queued applied inserts use latest revisions, publish, and reopen',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final base = _projectJson(revision: 7);
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: base,
        profile: AuthoringValidationProfile.experimental,
      );
      final core = _StoryCore(_appliedResponse);
      final builder = _RecordingMutationBuilder();
      final controller = StoryWorkspaceController(
        session: session,
        ffi: ModFfi(core),
        idSource: _QueuedIdSource(<StoryDraftEntityIds>[
          StoryDraftEntityIds(
            draftId: _firstDraftId,
            scriptModuleId: _firstModuleId,
          ),
          StoryDraftEntityIds(
            draftId: _secondDraftId,
            scriptModuleId: _secondModuleId,
          ),
        ]),
        mutationBuilder: builder,
      );
      final preparesBefore = store.prepareCalls;

      final firstFuture = controller.createNpc(_npcInput('First'));
      final secondFuture = controller.createNpc(_npcInput('Second'));
      final first = await firstFuture;
      final second = await secondFuture;

      expect(first, isA<StoryDraftCreateApplied>());
      expect(second, isA<StoryDraftCreateApplied>());
      final firstApplied = first as StoryDraftCreateApplied;
      final secondApplied = second as StoryDraftCreateApplied;
      expect(firstApplied.state.revision, 8);
      expect(firstApplied.draft.draftId, _firstDraftId);
      expect(firstApplied.draft.source, contains('GoreFirst'));
      expect(secondApplied.state.revision, 9);
      expect(secondApplied.state.drafts, hasLength(2));
      expect(secondApplied.draft.draftId, _secondDraftId);
      expect(secondApplied.draft.source, contains('GoreSecond'));
      expect(secondApplied.state.blocksBuild, isTrue);
      expect(store.prepareCalls, preparesBefore + 2);
      expect(builder.revisions, <int>[7, 8]);
      expect(core.calls, hasLength(2));
      expect(
        core.calls.map((call) => call.payload['profile']),
        everyElement('experimental'),
      );
      expect(
        (jsonDecode(core.calls[0].payload['mutation_json']! as String)
            as Map<String, Object?>)['expected_revision'],
        7,
      );
      expect(
        (jsonDecode(core.calls[1].payload['mutation_json']! as String)
            as Map<String, Object?>)['expected_revision'],
        8,
      );
      expect(controller.current.revision, 9);
      expect(controller.current.drafts, hasLength(2));
      final publishedHead = session.head.canonicalJson;
      await session.close();

      final reopened = await ManagedAuthoringProjectSession.open(
        root: root,
        store: store,
        profile: AuthoringValidationProfile.experimental,
      );
      final reopenedController = StoryWorkspaceController(
        session: reopened,
        ffi: ModFfi(core),
        idSource: _QueuedIdSource(<StoryDraftEntityIds>[]),
      );
      expect(reopened.head.canonicalJson, publishedHead);
      expect(reopenedController.current.revision, 9);
      expect(
        reopenedController.current.drafts.map((draft) => draft.draftId),
        <String>[_firstDraftId, _secondDraftId],
      );
      expect(
        reopenedController.current.drafts[1].source,
        contains('GoreSecond'),
      );
      expect(reopenedController.current.blocksBuild, isTrue);
      await reopened.close();
    },
  );

  test(
    'semantic rejection returns diagnostics with zero prepare or write',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final base = _projectJson(revision: 7);
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: base,
        profile: AuthoringValidationProfile.production,
      );
      final headBefore = await session.headFile.readAsBytes();
      final preparesBefore = store.prepareCalls;
      final core = _StoryCore(_rejectedResponse);
      final controller = StoryWorkspaceController(
        session: session,
        ffi: ModFfi(core),
        idSource: _QueuedIdSource(<StoryDraftEntityIds>[
          StoryDraftEntityIds(
            draftId: _firstDraftId,
            scriptModuleId: _firstModuleId,
          ),
        ]),
      );

      final result = await controller.createNpc(_npcInput('Rejected'));

      expect(result, isA<StoryDraftCreateRejected>());
      final rejected = result as StoryDraftCreateRejected;
      expect(rejected.state.revision, 7);
      expect(rejected.state.drafts, isEmpty);
      expect(rejected.diagnostics.single.code, 'PROJECT_REVISION_CONFLICT');
      expect(core.calls.single.payload['profile'], 'production');
      expect(store.prepareCalls, preparesBefore);
      expect(await session.headFile.readAsBytes(), headBefore);
      expect(session.projectJson, base);
      await session.close();
    },
  );

  test('readiness check is exact, read-only, and profile-bound', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final projectJson = _projectJson(revision: 7);
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: projectJson,
      profile: AuthoringValidationProfile.experimental,
    );
    final core = _BuildPlanCore();
    final controller = StoryWorkspaceController(
      session: session,
      ffi: ModFfi(core),
    );
    final preparesBefore = store.prepareCalls;
    final headBefore = await session.headFile.readAsBytes();

    final result = await controller.checkBuildPlan();

    expect(result, isA<StoryBuildReadinessChecked>());
    final checked = result as StoryBuildReadinessChecked;
    expect(checked.projectRevision, 7);
    expect(checked.moduleCount, 0);
    expect(checked.diagnosticCount, 1);
    expect(checked.blockingDiagnosticCount, 1);
    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, 'authoring_story_build_plan_v1_generate');
    expect(core.calls.single.payload, <String, Object?>{
      'project_json': projectJson,
      'profile': 'experimental',
    });
    expect(store.prepareCalls, preparesBefore);
    expect(session.projectJson, projectJson);
    expect(await session.headFile.readAsBytes(), headBefore);
    await session.close();
  });

  test(
    'readiness result is marked stale when the managed head drifts',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
        profile: AuthoringValidationProfile.production,
      );
      final entered = Completer<void>();
      final release = Completer<void>();
      final core = _BuildPlanCore(entered: entered, release: release);
      final controller = StoryWorkspaceController(
        session: session,
        ffi: ModFfi(core),
      );

      final pending = controller.checkBuildPlan();
      await entered.future;
      final externalHead = store.register(_projectJson(revision: 8));
      await session.headFile.writeAsString(
        externalHead.canonicalJson,
        flush: true,
      );
      release.complete();

      expect(await pending, isA<StoryBuildReadinessStale>());
      expect(session.requiresReopen, isTrue);
      expect(core.calls, hasLength(1));
      await session.close();
    },
  );

  test('stale head propagates before FFI and does not prepare', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 7),
      profile: AuthoringValidationProfile.experimental,
    );
    final core = _StoryCore(_appliedResponse);
    final controller = _controller(session, core);
    final preparesBefore = store.prepareCalls;
    final externalHead = store.register(_projectJson(revision: 99));
    await session.headFile.writeAsString(
      externalHead.canonicalJson,
      flush: true,
    );

    await expectLater(
      controller.createNpc(_npcInput('Stale')),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );
    expect(core.calls, isEmpty);
    expect(store.prepareCalls, preparesBefore);
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test('closed and reentrant session errors propagate unchanged', () async {
    final rootA = Directory(p.join(fixture.path, 'closed'));
    await rootA.create();
    final closedSession = await ManagedAuthoringProjectSession.create(
      root: rootA,
      store: _FakeManagedStore(),
      projectJson: _projectJson(revision: 7),
      profile: AuthoringValidationProfile.experimental,
    );
    final closedCore = _StoryCore(_appliedResponse);
    final closedController = _controller(closedSession, closedCore);
    await closedSession.close();
    await expectLater(
      closedController.createNpc(_npcInput('Closed')),
      throwsA(isA<ManagedProjectSessionClosedException>()),
    );
    expect(closedCore.calls, isEmpty);

    final rootB = Directory(p.join(fixture.path, 'reentrant'));
    await rootB.create();
    final reentrantSession = await ManagedAuthoringProjectSession.create(
      root: rootB,
      store: _FakeManagedStore(),
      projectJson: _projectJson(revision: 7),
      profile: AuthoringValidationProfile.experimental,
    );
    final reentrantCore = _StoryCore(_appliedResponse);
    final reentrantController = _controller(reentrantSession, reentrantCore);
    await expectLater(
      reentrantSession.deriveAndSave<void>((_) async {
        await reentrantController.createNpc(_npcInput('Nested'));
        return const ManagedProjectDerivedRejection<void>(null);
      }),
      throwsA(isA<ManagedProjectReentrantOperationException>()),
    );
    expect(reentrantCore.calls, isEmpty);
    expect(reentrantSession.requiresReopen, isFalse);
    await reentrantSession.close();
  });

  test('Story projection rejects noncanonical and unsealed source bytes', () {
    final base = _projectJson(revision: 7);
    final mutation = buildNpcStoryDraftMutationJson(
      context: StoryDraftMutationContext(
        projectId: _projectId,
        revision: 7,
        ids: StoryDraftEntityIds(
          draftId: _firstDraftId,
          scriptModuleId: _firstModuleId,
        ),
      ),
      input: _npcInput('Projection'),
    );
    final candidate = _candidateProject(base, mutation);

    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        '$candidate\n',
        blocksBuild: true,
        diagnostics: _combinedAuthoringDiagnostics(),
      ),
      throwsFormatException,
    );
    final corrupted = (jsonDecode(candidate) as Map).cast<String, Object?>();
    final entities = (corrupted['entities'] as Map).cast<String, Object?>();
    final module = (entities[_firstModuleId] as Map).cast<String, Object?>();
    final payload = (module['payload'] as Map).cast<String, Object?>();
    final data = (payload['data'] as Map).cast<String, Object?>();
    data['source_sha256'] = List<String>.filled(64, 'f').join();

    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        jsonEncode(corrupted),
        blocksBuild: true,
        diagnostics: _combinedAuthoringDiagnostics(),
      ),
      throwsFormatException,
    );

    final malformed = (jsonDecode(candidate) as Map).cast<String, Object?>();
    final malformedEntities = (malformed['entities'] as Map)
        .cast<String, Object?>();
    final malformedModule = (malformedEntities[_firstModuleId] as Map)
        .cast<String, Object?>();
    final malformedPayload = (malformedModule['payload'] as Map)
        .cast<String, Object?>();
    final malformedData = (malformedPayload['data'] as Map)
        .cast<String, Object?>();
    malformedData['source'] = String.fromCharCode(0xd800);
    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        jsonEncode(malformed),
        blocksBuild: true,
        diagnostics: _combinedAuthoringDiagnostics(),
      ),
      throwsFormatException,
    );
  });

  test('Story projection requires exact internally consistent build gate', () {
    final project = _projectJson(revision: 7);
    final valid = _combinedAuthoringDiagnostics();
    expect(
      StoryWorkspaceState.fromCanonicalProjectJson(
        project,
        blocksBuild: true,
        diagnostics: valid,
      ).blocksBuild,
      isTrue,
    );
    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        project,
        blocksBuild: false,
        diagnostics: valid,
      ),
      throwsFormatException,
    );
    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        project,
        blocksBuild: false,
        diagnostics: const <AuthoringDiagnostic>[],
      ),
      throwsFormatException,
    );
    final malformedGate = <String, Object?>{
      ..._combinedDiagnostic(),
      'severity': 'warning',
    };
    expect(
      () => StoryWorkspaceState.fromCanonicalProjectJson(
        project,
        blocksBuild: true,
        diagnostics: <AuthoringDiagnostic>[
          AuthoringDiagnostic.fromJson(malformedGate),
        ],
      ),
      throwsFormatException,
    );
  });
}

StoryWorkspaceController _controller(
  ManagedAuthoringProjectSession session,
  _StoryCore core,
) => StoryWorkspaceController(
  session: session,
  ffi: ModFfi(core),
  idSource: _QueuedIdSource(<StoryDraftEntityIds>[
    StoryDraftEntityIds(draftId: _firstDraftId, scriptModuleId: _firstModuleId),
  ]),
);

StoryNpcDraftInput _npcInput(String suffix) => StoryNpcDraftInput(
  displayName: 'NPC Gore$suffix',
  moduleNamespace: 'GoreMods.Npcs.Gore$suffix',
  uniqueName: 'Gore$suffix',
  parentCharacterDefinition: _trusted(
    _npcParent('02', 'CatalogCharacter$suffix', 'UCharacter$suffix'),
  ),
  parentAiAgentConfig: _trusted(
    _npcParent('03', 'CatalogAi$suffix', 'UAi$suffix'),
  ),
  parentSpawnDefinition: _trusted(
    _npcParent('04', 'CatalogSpawn$suffix', 'USpawn$suffix'),
  ),
);

CanonicalUnverifiedStoryJsonObject _trusted(Map<String, Object?> value) =>
    CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(jsonEncode(value));

Map<String, Object?> _generation() => <String, Object?>{
  'executable': <String, Object?>{
    'byte_len': 1000000,
    'sha256': List<String>.filled(64, '1').join(),
  },
};

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(32, byte).join(),
};

Map<String, Object?> _npcParent(
  String sealByte,
  String selector,
  String runtimeClass,
) => <String, Object?>{
  'generation': _generation(),
  'source_seal': _seal(sealByte, 20000),
  'catalog_layer': 'base-game.g1r.characters',
  'canonical_selector': selector,
  'runtime_class': runtimeClass,
};

String _projectJson({required int revision}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 2,
  'project_id': _projectId,
  'revision': revision,
  'meta': <String, Object?>{
    'name': 'Story controller',
    'version': '0.1',
    'author': 'tests',
  },
  'target': _generation(),
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _appliedResponse(Map<String, Object?> payload) {
  final projectJson = payload['project_json']! as String;
  final mutationJson = payload['mutation_json']! as String;
  final profile = payload['profile']! as String;
  final mutation = (jsonDecode(mutationJson) as Map).cast<String, Object?>();
  final candidate = _candidateProject(projectJson, mutationJson);
  final candidateMap = (jsonDecode(candidate) as Map).cast<String, Object?>();
  return <String, Object?>{
    'ok': true,
    'outcome': 'applied',
    'request_binding_sha256': _requestBinding(
      projectJson,
      mutationJson,
      profile,
    ),
    'project_json': candidate,
    'revision': candidateMap['revision'],
    'draft_id': mutation['draft_id'],
    'draft_kind': 'npc_draft',
    'script_module_id': mutation['script_module_id'],
    'diagnostics': <Object?>[_combinedDiagnostic()],
    'blocks_build': true,
  };
}

Map<String, Object?> _rejectedResponse(Map<String, Object?> payload) {
  final projectJson = payload['project_json']! as String;
  final mutationJson = payload['mutation_json']! as String;
  final profile = payload['profile']! as String;
  return <String, Object?>{
    'ok': true,
    'outcome': 'rejected',
    'request_binding_sha256': _requestBinding(
      projectJson,
      mutationJson,
      profile,
    ),
    'diagnostics': <Object?>[
      <String, Object?>{
        'code': 'PROJECT_REVISION_CONFLICT',
        'severity': 'error',
        'entity': null,
        'property_path': 'expected_revision',
        'message': 'injected semantic rejection',
        'related_entities': <Object?>[],
        'blocks_build': true,
      },
    ],
  };
}

String _candidateProject(String projectJson, String mutationJson) {
  final base = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final mutation = (jsonDecode(mutationJson) as Map).cast<String, Object?>();
  final draftRequest = (mutation['draft'] as Map).cast<String, Object?>();
  final requestInput = (draftRequest['input'] as Map).cast<String, Object?>();
  final draftId = mutation['draft_id']! as String;
  final moduleId = mutation['script_module_id']! as String;
  final namespace = requestInput['module_namespace']! as String;
  final runtimeId = requestInput['unique_name']! as String;
  final source = '// generated NPC $runtimeId\n';
  final owner = <String, Object?>{
    'project_id': _projectId,
    'id': draftId,
    'expected_kind': 'npc_draft',
  };
  final moduleRef = <String, Object?>{
    'project_id': _projectId,
    'id': moduleId,
    'expected_kind': 'script_module',
  };
  final additions = <String, Object?>{
    draftId: <String, Object?>{
      'id': draftId,
      'display_name': mutation['display_name'],
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': runtimeId,
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'input': <String, Object?>{'target': base['target'], ...requestInput},
          'script_module': moduleRef,
        },
      },
    },
    moduleId: <String, Object?>{
      'id': moduleId,
      'display_name': namespace,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.logical-npc-clone-draft',
        'generator_version': 1,
        'owner': owner,
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'owner': owner,
          'module_namespace': namespace,
          'module_relative_path': '${namespace.replaceAll('.', '/')}.as',
          'source': source,
          'source_sha256': crypto.sha256
              .convert(utf8.encode(source))
              .toString(),
          'input_fingerprint': List<String>.filled(64, 'a').join(),
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
    },
  };
  final existing = (base['entities'] as Map).cast<String, Object?>();
  final sortedEntities = SplayTreeMap<String, Object?>()
    ..addAll(existing)
    ..addAll(additions);
  base['revision'] = (base['revision']! as int) + 1;
  base['entities'] = <String, Object?>{
    for (final entry in sortedEntities.entries) entry.key: entry.value,
  };
  return jsonEncode(base);
}

String _requestBinding(
  String projectJson,
  String mutationJson,
  String profile,
) {
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore-authoring.story-draft-insert-v1.request-binding\u0000'),
    );
  for (final part in <List<int>>[
    utf8.encode(projectJson),
    utf8.encode(mutationJson),
    utf8.encode(profile),
  ]) {
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, part.length, Endian.little);
    bytes
      ..add(length)
      ..add(part);
  }
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Map<String, Object?> _combinedDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'entity': null,
  'property_path': 'schema_revision',
  'message':
      'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented',
  'related_entities': <Object?>[],
  'blocks_build': true,
};

List<AuthoringDiagnostic> _combinedAuthoringDiagnostics() =>
    <AuthoringDiagnostic>[AuthoringDiagnostic.fromJson(_combinedDiagnostic())];

typedef _StoryResponder =
    Map<String, Object?> Function(Map<String, Object?> payload);

final class _BuildPlanCore implements GoreCoreFfiService {
  _BuildPlanCore({this.entered, this.release});

  final Completer<void>? entered;
  final Completer<void>? release;
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  String get description => 'build-plan-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const <String, Object?>{},
  }) async {
    calls.add((command: command, payload: payload));
    if (command != 'authoring_story_build_plan_v1_generate') {
      throw StateError('unexpected command $command');
    }
    if (entered != null && !entered!.isCompleted) entered!.complete();
    if (release != null) await release!.future;
    return _buildPlanResponse(payload);
  }
}

Map<String, Object?> _buildPlanResponse(Map<String, Object?> payload) {
  final projectJson = payload['project_json']! as String;
  final profile = payload['profile']! as String;
  final rawProject = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final target = (rawProject['target'] as Map).cast<String, Object?>();
  final project = <String, Object?>{
    'project_id': rawProject['project_id'],
    'project_revision': rawProject['revision'],
    'canonical_document': _storyBuildSeal(projectJson),
    'target_executable': target['executable'],
  };
  final plan = <String, Object?>{
    'format': 'story_build_plan',
    'schema_revision': 1,
    'validation_profile': profile,
    'project': project,
    'publication_status': 'not_supported',
    'modules': <Object?>[],
    'diagnostics': <Object?>[_buildCombinedDiagnostic()],
    'blocks_build': true,
  };
  final planJson = jsonEncode(plan);
  return <String, Object?>{
    'ok': true,
    'request_binding_sha256': _buildPlanBinding(projectJson, profile),
    'plan_json': planJson,
    'plan_seal': _storyBuildSeal(planJson),
    'validation_profile': profile,
    'project': (jsonDecode(jsonEncode(project)) as Map).cast<String, Object?>(),
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'module_count': 0,
    'diagnostic_count': 1,
    'blocking_diagnostic_indexes': <Object?>[0],
    'blocks_build': true,
  };
}

Map<String, Object?> _buildCombinedDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'property_path': 'schema_revision',
  'message':
      'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented',
  'blocks_build': true,
};

Map<String, Object?> _storyBuildSeal(String value) {
  final bytes = utf8.encode(value);
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

String _buildPlanBinding(String projectJson, String profile) {
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore-story-build.authoring-plan-v1.request-binding\u0000'),
    );
  for (final part in <List<int>>[
    utf8.encode(projectJson),
    utf8.encode(profile),
  ]) {
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, part.length, Endian.little);
    bytes
      ..add(length)
      ..add(part);
  }
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

final class _StoryCore implements GoreCoreFfiService {
  _StoryCore(this._responder);

  final _StoryResponder _responder;
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  String get description => 'story-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const <String, Object?>{},
  }) async {
    calls.add((command: command, payload: payload));
    if (command != 'authoring_project_story_draft_insert_v1') {
      return <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'UNKNOWN_COMMAND',
          'message': command,
        },
      };
    }
    return _responder(payload);
  }
}

final class _QueuedIdSource implements StoryDraftIdSource {
  _QueuedIdSource(Iterable<StoryDraftEntityIds> ids)
    : _ids = Queue<StoryDraftEntityIds>.of(ids);

  final Queue<StoryDraftEntityIds> _ids;

  @override
  StoryDraftEntityIds next() {
    if (_ids.isEmpty) throw StateError('no test Story IDs remain');
    return _ids.removeFirst();
  }
}

final class _RecordingMutationBuilder implements StoryDraftMutationJsonBuilder {
  final List<int> revisions = <int>[];
  final ClosedStoryDraftMutationJsonBuilder _delegate =
      const ClosedStoryDraftMutationJsonBuilder();

  @override
  String buildNpc({
    required StoryDraftMutationContext context,
    required StoryNpcDraftInput input,
  }) {
    revisions.add(context.revision);
    return _delegate.buildNpc(context: context, input: input);
  }
}

final class _FakeManagedStore implements ManagedAuthoringStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  int _sequence = 0;
  int prepareCalls = 0;

  AuthoringWorkingHead register(String projectJson) {
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
    _projectsByHead[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final head = AuthoringWorkingHead.fromCanonicalJson(rawHead);
    final project = _projectsByHead[rawHead];
    if (project == null) throw StateError('unknown published head');
    return _opened(head, project);
  }

  @override
  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    final project = _projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown checkpoint head');
    return _opened(head, project);
  }

  @override
  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    prepareCalls++;
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_document_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake Story head conflict',
      );
    }
    final head = register(projectJson);
    return AuthoringCheckpointPreparation.fromJson(_preparedResponse(head));
  }

  AuthoringStoreOpenedResult _opened(
    AuthoringWorkingHead head,
    String projectJson,
  ) => AuthoringStoreOpenedResult.fromJson(<String, Object?>{
    ..._preparedResponse(head),
    'project_json': projectJson,
  });
}

Map<String, Object?> _preparedResponse(AuthoringWorkingHead head) =>
    <String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
      'diagnostics': <Object?>[_combinedDiagnostic()],
      'blocks_build': true,
    };
