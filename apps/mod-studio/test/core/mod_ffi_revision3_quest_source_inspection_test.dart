import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _root = r'C:\Projects\QuestInspection.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '00000000000000000000000000000031';
const _questId = '00000000000000000000000000000071';
const _moduleId = '00000000000000000000000000000072';
const _otherId = '00000000000000000000000000000073';
const _projectRevision = 12;
const _maxProjectBytes = 16 * 1024 * 1024;
const _source = '''class UQuest_GoreInspection : UQuest
{
    void OnStart() {}
}
''';

String _sha(String value) =>
    crypto.sha256.convert(utf8.encode(value)).toString();

Map<String, Object?> _seal(int byteLength, String sha256) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
};

Map<String, Object?> _digitSeal(int byteLength, String digit) =>
    _seal(byteLength, List<String>.filled(64, digit).join());

Map<String, Object?> _projectSeal() => _digitSeal(2048, '1');

Map<String, Object?> _headObject([String digit = 'a']) => <String, Object?>{
  'store_format': 1,
  'snapshot': _digitSeal(4096, digit),
};

String _headJson([String digit = 'a']) => jsonEncode(_headObject(digit));

Map<String, Object?> _typedRef(String id, String kind) => <String, Object?>{
  'project_id': _projectId,
  'id': id,
  'expected_kind': kind,
};

Map<String, Object?> _validPlan() {
  final sourceBytes = utf8.encode(_source);
  final sourceSha = crypto.sha256.convert(sourceBytes).toString();
  return <String, Object?>{
    'format': 'revision3_quest_source_inspection_plan',
    'schema_revision': 3,
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': _projectId,
      'project_revision': _projectRevision,
      'target_executable': _digitSeal(128 * 1024 * 1024, '2'),
      'canonical_project': _projectSeal(),
      'collision_basis_head': _headObject('b'),
      'collision_basis_project': _digitSeal(1024, '3'),
      'collision_nonquest_project': _digitSeal(900, '4'),
      'collision_prior_quest_count': 2,
      'collision_prior_quest_evidence': _digitSeal(300, '5'),
      'collision_artifact': _digitSeal(700, '6'),
      'collision_source': _digitSeal(700, '7'),
    },
    'module': <String, Object?>{
      'quest': _typedRef(_questId, 'quest_draft'),
      'script_module': _typedRef(_moduleId, 'script_module'),
      'draft_input': _digitSeal(420, '8'),
      'persisted_source': _seal(sourceBytes.length, sourceSha),
      'generated': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': _typedRef(_questId, 'quest_draft'),
        'module_namespace': 'GoreMods.Quests.Inspection',
        'module_relative_path': 'GoreMods/Quests/Inspection.as',
        'source': _source,
        'source_sha256': sourceSha,
        'input_fingerprint': List<String>.filled(64, '9').join(),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  };
}

void _setPlanJson(Map<String, Object?> response, String planJson) {
  final bytes = utf8.encode(planJson);
  response['plan_json'] = planJson;
  response['plan_seal'] = _seal(
    bytes.length,
    crypto.sha256.convert(bytes).toString(),
  );
}

Map<String, Object?> _validResponse() {
  final response = <String, Object?>{
    'ok': true,
    'outcome': 'inspection_only',
    'head_json': _headJson(),
    'project_id': _projectId,
    'project_revision': _projectRevision,
    'project_seal': _projectSeal(),
    'quest_id': _questId,
    'plan_json': '',
    'plan_seal': _digitSeal(1, 'f'),
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
  _setPlanJson(response, jsonEncode(_validPlan()));
  return response;
}

void _mutatePlan(
  Map<String, Object?> response,
  void Function(Map<String, Object?> plan) mutate,
) {
  final plan = (jsonDecode(response['plan_json']! as String) as Map)
      .cast<String, Object?>();
  mutate(plan);
  _setPlanJson(response, jsonEncode(plan));
}

Future<void> _expectMalformed(Map<String, Object?> response) async {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_store_inspect_revision3_quest_source_v1': response,
    },
  );
  await expectLater(
    ModFfi(core).authoringStoreInspectRevision3QuestSourceV1(
      root: _root,
      gameRoot: _gameRoot,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
      questId: _questId,
    ),
    throwsA(
      isA<ModFfiException>().having(
        (error) => error.code,
        'code',
        ModFfiException.malformedNativeResponseCode,
      ),
    ),
  );
}

void main() {
  test('Studio handshake requires the sorted Quest inspection command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_inspect_revision3_quest_source_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'FFI sends only the exact read-only request and parses PlanV3',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_inspect_revision3_quest_source_v1': _validResponse(),
        },
      );
      final expectedHead = AuthoringWorkingHead.fromCanonicalJson(_headJson());

      final result = await ModFfi(core)
          .authoringStoreInspectRevision3QuestSourceV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: expectedHead,
            questId: _questId,
          );

      expect(
        core.calls.single.command,
        'authoring_store_inspect_revision3_quest_source_v1',
      );
      expect(core.calls.single.payload, <String, Object?>{
        'root': _root,
        'game_root': _gameRoot,
        'expected_head_json': _headJson(),
        'quest_id': _questId,
      });
      expect(result.head.canonicalJson, _headJson());
      expect(result.projectId, _projectId);
      expect(result.projectRevision, _projectRevision);
      expect(result.projectSeal.byteLength, 2048);
      expect(result.questId, _questId);
      expect(
        result.scope,
        AuthoringRevision3QuestInspectionScope.sourceInspectionOnly,
      );
      expect(
        result.buildStatus,
        AuthoringRevision3QuestInspectionBuildStatus.blocked,
      );
      expect(
        result.runtimeQualification,
        AuthoringRevision3QuestInspectionRuntimeQualification
            .runtimeUnqualified,
      );
      expect(
        result.publicationStatus,
        AuthoringRevision3QuestInspectionPublicationStatus.notSupported,
      );
      expect(result.generatedSource, _source);
      expect(result.moduleNamespace, 'GoreMods.Quests.Inspection');
      expect(result.moduleRelativePath, 'GoreMods/Quests/Inspection.as');
      expect(result.plan.module.quest.id, _questId);
      expect(result.plan.module.scriptModule.id, _moduleId);
      expect(result.plan.module.generated.generatorVersion, 4);
      expect(result.plan.provenance.collisionPriorQuestCount, 2);
      expect(
        result.plan.provenance.collisionBasisHead.canonicalJson,
        _headJson('b'),
      );
      expect(result.planSeal.byteLength, utf8.encode(result.planJson).length);
      expect(result.planSeal.sha256, _sha(result.planJson));
    },
  );

  test(
    'response rejects unknown, missing, widened, or unbound fields',
    () async {
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response['authority'] = 'compile',
        (response) => response.remove('plan_seal'),
        (response) => response['outcome'] = 'build_ready',
        (response) => response['head_json'] = _headJson('c'),
        (response) => response['project_id'] = _otherId,
        (response) => response['project_revision'] = _projectRevision + 1,
        (response) => response['project_seal'] = _digitSeal(2048, 'e'),
        (response) => response['quest_id'] = _otherId,
        (response) => response['plan_seal'] = _digitSeal(1, 'f'),
        (response) => response['scope'] = 'compile_and_run',
        (response) => response['build_status'] = 'ready',
        (response) => response['runtime_qualification'] = 'runtime_qualified',
        (response) => response['publication_status'] = 'supported',
      ];

      for (final mutate in mutations) {
        final response = _validResponse();
        mutate(response);
        await _expectMalformed(response);
      }
    },
  );

  test(
    'PlanV3 rejects schema drift, authority claims, and broken bindings',
    () async {
      final mutations = <void Function(Map<String, Object?>)>[
        (plan) => plan['extra'] = true,
        (plan) => plan['schema_revision'] = 2,
        (plan) => plan['scope'] = 'compile_and_run',
        (plan) => plan['build_status'] = 'ready',
        (plan) => plan['runtime_qualification'] = 'runtime_qualified',
        (plan) => plan['publication_status'] = 'supported',
        (plan) =>
            (plan['provenance']! as Map<String, Object?>)['extra'] = 'forged',
        (plan) =>
            (plan['provenance']!
                    as Map<String, Object?>)['collision_prior_quest_count'] =
                14286,
        (plan) =>
            (plan['provenance']! as Map<String, Object?>)['collision_source'] =
                _digitSeal(701, '7'),
        (plan) => (plan['module']! as Map<String, Object?>)['extra'] = true,
        (plan) =>
            ((plan['module']! as Map<String, Object?>)['quest']!
                    as Map<String, Object?>)['expected_kind'] =
                'script_module',
        (plan) =>
            ((plan['module']! as Map<String, Object?>)['script_module']!
                    as Map<String, Object?>)['id'] =
                _questId,
        (plan) =>
            (((plan['module']! as Map<String, Object?>)['generated']!
                        as Map<String, Object?>)['owner']!
                    as Map<String, Object?>)['id'] =
                _otherId,
        (plan) =>
            ((plan['module']! as Map<String, Object?>)['generated']!
                    as Map<String, Object?>)['module_relative_path'] =
                'GoreMods/Quests/Wrong.as',
        (plan) =>
            ((plan['module']! as Map<String, Object?>)['generated']!
                    as Map<String, Object?>)['source'] =
                'tampered source',
        (plan) {
          final module = plan['module']! as Map<String, Object?>;
          final generated = module['generated']! as Map<String, Object?>;
          final status = generated['status']! as Map<String, Object?>;
          status['runtime'] = 'runtime_qualified';
        },
      ];

      for (final mutate in mutations) {
        final response = _validResponse();
        _mutatePlan(response, mutate);
        await _expectMalformed(response);
      }
    },
  );

  test(
    'prior-Quest evidence accepts the Rust project boundary and rejects one byte over',
    () async {
      final boundaryResponse = _validResponse();
      _mutatePlan(boundaryResponse, (plan) {
        final provenance = plan['provenance']! as Map<String, Object?>;
        provenance['collision_prior_quest_evidence'] = _digitSeal(
          _maxProjectBytes,
          '5',
        );
      });
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_inspect_revision3_quest_source_v1': boundaryResponse,
        },
      );
      final result = await ModFfi(core)
          .authoringStoreInspectRevision3QuestSourceV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
            questId: _questId,
          );
      expect(
        result.plan.provenance.collisionPriorQuestEvidence.byteLength,
        _maxProjectBytes,
      );

      final oversizedResponse = _validResponse();
      _mutatePlan(oversizedResponse, (plan) {
        final provenance = plan['provenance']! as Map<String, Object?>;
        provenance['collision_prior_quest_evidence'] = _digitSeal(
          _maxProjectBytes + 1,
          '5',
        );
      });
      await _expectMalformed(oversizedResponse);
    },
  );

  test(
    'PlanV3 requires Rust canonical field order and duplicate-free JSON',
    () async {
      final response = _validResponse();
      final plan = _validPlan();
      final reordered = <String, Object?>{
        'schema_revision': plan['schema_revision'],
        'format': plan['format'],
        'scope': plan['scope'],
        'build_status': plan['build_status'],
        'runtime_qualification': plan['runtime_qualification'],
        'publication_status': plan['publication_status'],
        'provenance': plan['provenance'],
        'module': plan['module'],
      };
      _setPlanJson(response, jsonEncode(reordered));
      await _expectMalformed(response);

      final nestedOrderResponse = _validResponse();
      _mutatePlan(nestedOrderResponse, (nestedPlan) {
        final provenance = nestedPlan['provenance']! as Map<String, Object?>;
        final seal = provenance['canonical_project']! as Map<String, Object?>;
        provenance['canonical_project'] = <String, Object?>{
          'sha256': seal['sha256'],
          'byte_len': seal['byte_len'],
        };
      });
      await _expectMalformed(nestedOrderResponse);

      final duplicateResponse = _validResponse();
      final canonical = duplicateResponse['plan_json']! as String;
      final duplicate = canonical.replaceFirst(
        '"format":"revision3_quest_source_inspection_plan",',
        '"format":"revision3_quest_source_inspection_plan",'
            '"format":"revision3_quest_source_inspection_plan",',
      );
      _setPlanJson(duplicateResponse, duplicate);
      await _expectMalformed(duplicateResponse);
    },
  );

  test(
    'request rejects invalid paths and non-canonical Quest IDs before native',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_inspect_revision3_quest_source_v1': _validResponse(),
        },
      );
      final ffi = ModFfi(core);
      final expectedHead = AuthoringWorkingHead.fromCanonicalJson(_headJson());
      for (final request
          in <Future<AuthoringRevision3QuestSourceInspectionResult> Function()>[
            () => ffi.authoringStoreInspectRevision3QuestSourceV1(
              root: _root,
              gameRoot: _gameRoot,
              expectedHead: expectedHead,
              questId: List<String>.filled(32, '0').join(),
            ),
            () => ffi.authoringStoreInspectRevision3QuestSourceV1(
              root: _root,
              gameRoot: _gameRoot,
              expectedHead: expectedHead,
              questId: 'A${_questId.substring(1)}',
            ),
            () => ffi.authoringStoreInspectRevision3QuestSourceV1(
              root: '$_root\u0000forged',
              gameRoot: _gameRoot,
              expectedHead: expectedHead,
              questId: _questId,
            ),
            () => ffi.authoringStoreInspectRevision3QuestSourceV1(
              root: _root,
              gameRoot: '',
              expectedHead: expectedHead,
              questId: _questId,
            ),
          ]) {
        await expectLater(
          request(),
          throwsA(anyOf(isA<FormatException>(), isA<ArgumentError>())),
        );
      }
      expect(core.calls, isEmpty);
    },
  );
}
