import 'dart:collection';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_npc_fixture.dart';

const _projectId = '00000000000000000000000000000003';
const _npcId = '00000000000000000000000000000081';
const _moduleId = '00000000000000000000000000000082';
const _executableByteLength = 171698176;
const _executableSha256 =
    'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5';
const _hotfixExecutableByteLength = 171704320;
const _hotfixExecutableSha256 =
    'b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d';
const _build24340829ExecutableByteLength = 171787776;
const _build24340829ExecutableSha256 =
    'ab2c8d9e286a437bc5343748faf40959a77e9dc7c542ff9361f1ffaeca5c811c';
const _build24878692ExecutableByteLength = 171792384;
const _build24878692ExecutableSha256 =
    '824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0';

String _headJson(String byte) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 321,
    'sha256': List<String>.filled(64, byte).join(),
  },
});

String _projectJson({
  int revision = 7,
  int executableByteLength = _executableByteLength,
  String executableSha256 = _executableSha256,
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': revision,
  'meta': <String, Object?>{
    'name': 'R3 NPC adapter',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': executableByteLength,
      'sha256': executableSha256,
    },
  },
  'authoring_locales': <Object?>[],
  'entities': SplayTreeMap<String, Object?>(),
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

AuthoringRevision3NpcDraftIntentV1 _intent({
  String parentCatalogId = 'g1r:npc:om_grd_asghan_263',
}) => AuthoringRevision3NpcDraftIntentV1(
  moduleNamespace: 'GoreMods.Npcs.ManagedGuard',
  uniqueName: 'GoreManagedGuard',
  parentCatalogId: parentCatalogId,
);

AuthoringRevision3NpcDraftRequestV1 _request({
  String? projectJson,
  String parentCatalogId = 'g1r:npc:om_grd_asghan_263',
}) => AuthoringRevision3NpcDraftRequestV1.forProject(
  expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('a')),
  currentProjectJson: projectJson ?? _projectJson(),
  npcId: _npcId,
  scriptModuleId: _moduleId,
  displayName: 'Managed Guard',
  intent: _intent(parentCatalogId: parentCatalogId),
);

Revision3NpcFixture _fixture({
  AuthoringRevision3NpcDraftRequestV1? forRequest,
}) {
  final request = forRequest ?? _request();
  return Revision3NpcFixture.fromBasis(
    basisHead: request.expectedHead,
    basisProjectJson: _projectJson(),
    request: request,
  );
}

void main() {
  test('required command handshake includes native R3 NPC preparation', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_npc_draft_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'NPC request derives and preserves the exact canonical project target',
    () {
      final request = _request();
      final reopened = AuthoringRevision3NpcDraftRequestV1.fromCanonicalJson(
        request.canonicalJson,
      );

      expect(reopened.expectedHead.canonicalJson, _headJson('a'));
      expect(reopened.expectedProjectId, _projectId);
      expect(reopened.expectedRevision, 7);
      expect(
        reopened.expectedTargetCanonicalJson,
        jsonEncode(
          (jsonDecode(_projectJson()) as Map<String, Object?>)['target'],
        ),
      );
      expect(reopened.npcId, _npcId);
      expect(reopened.scriptModuleId, _moduleId);
      expect(reopened.intent.parentCatalogId, 'g1r:npc:om_grd_asghan_263');

      final raw = (jsonDecode(request.canonicalJson) as Map)
          .cast<String, Object?>();
      final reordered = <String, Object?>{
        'expected_project_id': raw['expected_project_id'],
        for (final entry in raw.entries)
          if (entry.key != 'expected_project_id') entry.key: entry.value,
      };
      final malformed = <String>[
        ' ${request.canonicalJson}',
        '${request.canonicalJson}\n',
        request.canonicalJson.replaceFirst(
          '"expected_revision":7',
          '"expected_revision":7,"expected_revision":7',
        ),
        jsonEncode(<String, Object?>{...raw, 'authority': 'forged'}),
        jsonEncode(<String, Object?>{...raw}..remove('expected_target')),
        jsonEncode(<String, Object?>{...raw, 'npc_id': _moduleId}),
        request.canonicalJson.replaceFirst(_npcId, List.filled(32, '0').join()),
        request.canonicalJson.replaceFirst(
          '"expected_revision":7',
          '"expected_revision":9223372036854775807',
        ),
        request.canonicalJson.replaceFirst(
          'g1r:npc:om_grd_asghan_263',
          'G1R:NPC:ASGHAN',
        ),
        request.canonicalJson.replaceFirst(
          'GoreMods.Npcs.ManagedGuard',
          r'\ud800',
        ),
        jsonEncode(reordered),
      ];
      for (final value in malformed) {
        expect(
          () => AuthoringRevision3NpcDraftRequestV1.fromCanonicalJson(value),
          throwsFormatException,
          reason: value.substring(0, value.length.clamp(0, 120)),
        );
      }
    },
  );

  test('frozen native Asghan and Viper vectors cross the strict boundary', () {
    for (final catalogId in const <String>[
      'g1r:npc:om_grd_asghan_263',
      'g1r:npc:om_stt_viper_302',
    ]) {
      final request = _request(parentCatalogId: catalogId);
      final fixture = _fixture(forRequest: request);
      final prepared = AuthoringRevision3NpcDraftPreparation.fromJson(
        fixture.response(),
        currentProjectJson: _projectJson(),
        request: request,
      );
      expect(prepared.parentCatalogId, catalogId);

      final project = (jsonDecode(prepared.projectJson) as Map)
          .cast<String, Object?>();
      final entities = (project['entities']! as Map).cast<String, Object?>();
      final npc = (entities[_npcId]! as Map).cast<String, Object?>();
      final payload = (npc['payload']! as Map).cast<String, Object?>();
      final data = (payload['data']! as Map).cast<String, Object?>();
      final input = (data['input']! as Map).cast<String, Object?>();
      for (final field in const <String>[
        'parent_character_definition',
        'parent_ai_agent_config',
        'parent_spawn_definition',
      ]) {
        final parent = (input[field]! as Map).cast<String, Object?>();
        expect(parent['catalog_layer'], revision3NpcFixtureCatalogLayer);
        expect(parent['canonical_selector'], startsWith('Catalog_'));
        expect((parent['canonical_selector']! as String).length, 72);
      }
    }
  });

  test(
    'later generations cross with the identical curated parent evidence',
    () {
      for (final generation in const <({int byteLength, String sha256})>[
        (
          byteLength: _hotfixExecutableByteLength,
          sha256: _hotfixExecutableSha256,
        ),
        (
          byteLength: _build24340829ExecutableByteLength,
          sha256: _build24340829ExecutableSha256,
        ),
        (
          byteLength: _build24878692ExecutableByteLength,
          sha256: _build24878692ExecutableSha256,
        ),
      ]) {
        final projectJson = _projectJson(
          executableByteLength: generation.byteLength,
          executableSha256: generation.sha256,
        );
        final request = _request(projectJson: projectJson);
        final fixture = Revision3NpcFixture.fromBasis(
          basisHead: request.expectedHead,
          basisProjectJson: projectJson,
          request: request,
        );

        final prepared = AuthoringRevision3NpcDraftPreparation.fromJson(
          fixture.response(),
          currentProjectJson: projectJson,
          request: request,
        );
        final project = (jsonDecode(prepared.projectJson) as Map)
            .cast<String, Object?>();
        final entities = (project['entities']! as Map).cast<String, Object?>();
        final npc = (entities[_npcId]! as Map).cast<String, Object?>();
        final payload = (npc['payload']! as Map).cast<String, Object?>();
        final data = (payload['data']! as Map).cast<String, Object?>();
        final input = (data['input']! as Map).cast<String, Object?>();
        expect(input['target'], project['target'], reason: generation.sha256);
        for (final field in const <String>[
          'parent_character_definition',
          'parent_ai_agent_config',
          'parent_spawn_definition',
        ]) {
          final parent = (input[field]! as Map).cast<String, Object?>();
          expect(
            parent['generation'],
            project['target'],
            reason: generation.sha256,
          );
          expect(
            parent['catalog_layer'],
            revision3NpcFixtureCatalogLayer,
            reason: generation.sha256,
          );
        }
      }
    },
  );

  test('nearby and cross-paired generation seals remain rejected', () {
    final unsupported = <({int byteLength, String sha256})>[
      (
        byteLength: _hotfixExecutableByteLength - 1,
        sha256: _hotfixExecutableSha256,
      ),
      (
        byteLength: _hotfixExecutableByteLength,
        sha256:
            '${_hotfixExecutableSha256.substring(0, 63)}${_hotfixExecutableSha256.endsWith('0') ? '1' : '0'}',
      ),
      (byteLength: _executableByteLength, sha256: _hotfixExecutableSha256),
      (byteLength: _hotfixExecutableByteLength, sha256: _executableSha256),
      (
        byteLength: _build24340829ExecutableByteLength - 1,
        sha256: _build24340829ExecutableSha256,
      ),
      (
        byteLength: _build24340829ExecutableByteLength,
        sha256:
            '${_build24340829ExecutableSha256.substring(0, 63)}${_build24340829ExecutableSha256.endsWith('0') ? '1' : '0'}',
      ),
      (
        byteLength: _hotfixExecutableByteLength,
        sha256: _build24340829ExecutableSha256,
      ),
      (
        byteLength: _build24340829ExecutableByteLength,
        sha256: _hotfixExecutableSha256,
      ),
      (
        byteLength: _build24878692ExecutableByteLength - 1,
        sha256: _build24878692ExecutableSha256,
      ),
      (
        byteLength: _build24878692ExecutableByteLength,
        sha256:
            '${_build24878692ExecutableSha256.substring(0, 63)}${_build24878692ExecutableSha256.endsWith('0') ? '1' : '0'}',
      ),
      (
        byteLength: _build24340829ExecutableByteLength,
        sha256: _build24878692ExecutableSha256,
      ),
      (
        byteLength: _build24878692ExecutableByteLength,
        sha256: _build24340829ExecutableSha256,
      ),
    ];

    for (final generation in unsupported) {
      final projectJson = _projectJson(
        executableByteLength: generation.byteLength,
        executableSha256: generation.sha256,
      );
      final request = _request(projectJson: projectJson);
      final fixture = Revision3NpcFixture.fromBasis(
        basisHead: request.expectedHead,
        basisProjectJson: projectJson,
        request: request,
      );
      expect(
        () => AuthoringRevision3NpcDraftPreparation.fromJson(
          fixture.response(),
          currentProjectJson: projectJson,
          request: request,
        ),
        throwsFormatException,
        reason: '${generation.byteLength}/${generation.sha256}',
      );
    }
  });

  test(
    'coherent parent evidence transplant to a foreign generation is rejected',
    () {
      final foreignProject = _projectJson(
        executableByteLength: 123456,
        executableSha256: List<String>.filled(64, '5').join(),
      );
      final request = _request(projectJson: foreignProject);
      final fixture = Revision3NpcFixture.fromBasis(
        basisHead: request.expectedHead,
        basisProjectJson: foreignProject,
        request: request,
      );

      expect(
        () => AuthoringRevision3NpcDraftPreparation.fromJson(
          fixture.response(),
          currentProjectJson: foreignProject,
          request: request,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'wrapper sends exact nested request and accepts only closed statuses',
    () async {
      final request = _request();
      final fixture = _fixture();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_npc_draft_v1': fixture.response(),
        },
      );
      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3NpcDraftV1(
            root: r'C:\Mods\Managed NPC.goreproj',
            gameRoot: r'D:\Games\Gothic Remake',
            currentProjectJson: _projectJson(),
            request: request,
          );

      expect(
        prepared.basisHead.canonicalJson,
        request.expectedHead.canonicalJson,
      );
      expect(prepared.head.canonicalJson, fixture.candidateHead.canonicalJson);
      expect(prepared.projectJson, fixture.candidateProjectJson);
      expect(prepared.projectId, _projectId);
      expect(prepared.revision, 8);
      expect(prepared.npcId, _npcId);
      expect(prepared.scriptModuleId, _moduleId);
      expect(prepared.displayName, 'Managed Guard');
      expect(prepared.moduleNamespace, 'GoreMods.Npcs.ManagedGuard');
      expect(prepared.uniqueName, 'GoreManagedGuard');
      expect(prepared.buildStatus, AuthoringRevision3NpcBuildStatus.blocked);
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3NpcRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.catalogAuthority,
        AuthoringRevision3NpcCatalogAuthority.notGranted,
      );
      expect(
        prepared.collisionAuthority,
        AuthoringRevision3NpcCollisionAuthority.notGranted,
      );
      expect(
        prepared.sourceInspection,
        AuthoringRevision3NpcSourceInspection.freshNativeContextRequired,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3NpcNativePublicationStatus.notSupported,
      );
      expect(core.calls, hasLength(1));
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': _projectJson(),
        'game_root': r'D:\Games\Gothic Remake',
        'npc_request_json': request.canonicalJson,
        'root': r'C:\Mods\Managed NPC.goreproj',
      });
    },
  );

  test(
    'preparation rejects loose claims and any broken NPC/module closure',
    () {
      final request = _request();
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response.remove('basis_head_json'),
        (response) => response['unknown'] = true,
        (response) => response['outcome'] = 'published',
        (response) => response['head_json'] = _headJson('a'),
        (response) => response['revision'] = 7,
        (response) => response['npc_id'] = _moduleId,
        (response) => response['build_status'] = 'ready',
        (response) => response['runtime_status'] = 'qualified',
        (response) => response['catalog_authority'] = 'granted',
        (response) => response['collision_authority'] = 'granted',
        (response) => response['source_inspection'] = 'available',
        (response) => response['publication_status'] = 'published',
        (response) =>
            response['project_json'] = (response['project_json']! as String)
                .replaceFirst('GoreMods.Npcs.ManagedGuard', r'\ud800'),
        (response) => response['project_json'] = _coherentlySwapParentSelection(
          response,
          replacementCatalogId: 'g1r:npc:om_stt_viper_302',
        ),
        (response) => response['project_json'] =
            _coherentlyForgeParentRuntimeClasses(response),
        (response) => response['project_json'] = _mutateProject(
          response,
          (project) => project['meta'] = <String, Object?>{
            'name': 'Changed',
            'version': '1.0.0',
            'author': 'tests',
          },
        ),
        (response) => response['project_json'] = _mutateNpcData(
          response,
          (data) => data['generator_version'] = 2,
        ),
        (response) => response['project_json'] = _mutateNpcInput(
          response,
          (input) => input['unique_name'] = 'OtherGuard',
        ),
        (response) =>
            response['project_json'] = _mutateNpcInput(response, (input) {
              final parent = (input['parent_character_definition'] as Map)
                  .cast<String, Object?>();
              parent['generation'] = <String, Object?>{
                'executable': <String, Object?>{
                  'byte_len': 1,
                  'sha256': List<String>.filled(64, '9').join(),
                },
              };
            }),
        (response) => response['project_json'] = _mutateModuleData(
          response,
          (data) => data['source'] = '// forged\n',
        ),
        (response) => response['project_json'] = _mutateModuleData(
          response,
          (data) => data['input_fingerprint'] = List.filled(64, '0').join(),
        ),
        (response) =>
            response['project_json'] = _mutateModuleData(response, (data) {
              final owner = (data['owner'] as Map).cast<String, Object?>();
              owner['expected_kind'] = 'quest_draft';
            }),
      ];
      for (final mutate in mutations) {
        final response = _fixture().response();
        mutate(response);
        expect(
          () => AuthoringRevision3NpcDraftPreparation.fromJson(
            response,
            currentProjectJson: _projectJson(),
            request: request,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'wrapper rejects project/request drift before any native call',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{},
      );
      final ffi = ModFfi(core);
      final request = _request();

      await expectLater(
        ffi.authoringStorePrepareRevision3NpcDraftV1(
          root: 'root',
          gameRoot: 'game',
          currentProjectJson: _projectJson(revision: 6),
          request: request,
        ),
        throwsFormatException,
      );
      await expectLater(
        ffi.authoringStorePrepareRevision3NpcDraftV1(
          root: 'root\u0000tail',
          gameRoot: 'game',
          currentProjectJson: _projectJson(),
          request: request,
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    },
  );

  test(
    'wrapper normalizes a malformed NPC response into poisonable code',
    () async {
      final response = _fixture().response()..['runtime_status'] = 'qualified';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_npc_draft_v1': response,
        },
      );

      await expectLater(
        ModFfi(core).authoringStorePrepareRevision3NpcDraftV1(
          root: 'root',
          gameRoot: 'game',
          currentProjectJson: _projectJson(),
          request: _request(),
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );
    },
  );
}

String _mutateProject(
  Map<String, Object?> response,
  void Function(Map<String, Object?> project) mutate,
) {
  final project = (jsonDecode(response['project_json']! as String) as Map)
      .cast<String, Object?>();
  mutate(project);
  return jsonEncode(project);
}

String _mutateNpcData(
  Map<String, Object?> response,
  void Function(Map<String, Object?> data) mutate,
) => _mutateProject(response, (project) {
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final entity = (entities[_npcId]! as Map).cast<String, Object?>();
  final payload = (entity['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  mutate(data);
});

String _mutateNpcInput(
  Map<String, Object?> response,
  void Function(Map<String, Object?> input) mutate,
) => _mutateNpcData(response, (data) {
  final input = (data['input']! as Map).cast<String, Object?>();
  mutate(input);
});

String _mutateModuleData(
  Map<String, Object?> response,
  void Function(Map<String, Object?> data) mutate,
) => _mutateProject(response, (project) {
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final entity = (entities[_moduleId]! as Map).cast<String, Object?>();
  final payload = (entity['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  mutate(data);
});

String _coherentlySwapParentSelection(
  Map<String, Object?> response, {
  required String replacementCatalogId,
}) => _coherentlyMutateParents(response, (input) {
  final target = (input['target']! as Map).cast<String, Object?>();
  final replacement = revision3NpcFixtureInput(
    request: _request(parentCatalogId: replacementCatalogId),
    target: target,
  );
  for (final field in const <String>[
    'parent_character_definition',
    'parent_ai_agent_config',
    'parent_spawn_definition',
  ]) {
    input[field] = replacement[field];
  }
});

String _coherentlyForgeParentRuntimeClasses(Map<String, Object?> response) =>
    _coherentlyMutateParents(response, (input) {
      final classes = <String, String>{
        'parent_character_definition':
            'UCharacterDefinition_Human_OM_STT_Viper_302',
        'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_STT_Viper_302',
        'parent_spawn_definition': 'USpawnAIAgentDefinition_OM_STT_Viper_302',
      };
      for (final entry in classes.entries) {
        final parent = (input[entry.key]! as Map).cast<String, Object?>();
        parent['runtime_class'] = entry.value;
      }
    });

String _coherentlyMutateParents(
  Map<String, Object?> response,
  void Function(Map<String, Object?> input) mutate,
) => _mutateProject(response, (project) {
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final npc = (entities[_npcId]! as Map).cast<String, Object?>();
  final npcPayload = (npc['payload']! as Map).cast<String, Object?>();
  final npcData = (npcPayload['data']! as Map).cast<String, Object?>();
  final input = (npcData['input']! as Map).cast<String, Object?>();
  mutate(input);

  final module = (entities[_moduleId]! as Map).cast<String, Object?>();
  final modulePayload = (module['payload']! as Map).cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  final source = revision3NpcFixtureSource(input);
  moduleData['source'] = source;
  moduleData['source_sha256'] = crypto.sha256
      .convert(utf8.encode(source))
      .toString();
  moduleData['input_fingerprint'] = revision3NpcFixtureInputFingerprint(input);
});
