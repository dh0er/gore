import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _root = r'C:\Projects\NpcInspection.goreproj';
const _projectId = '00000000000000000000000000000031';
const _npcId = '00000000000000000000000000000051';
const _moduleId = '00000000000000000000000000000052';
const _otherId = '00000000000000000000000000000053';
const _revision = 7;
const _generator = 'gore-authoring.logical-npc-clone-draft';
const _namespace = 'GoreMods.Npcs.GateGuard';
const _uniqueName = 'GORE_GateGuard';

Map<String, Object?> _seal(int length, String digit) => <String, Object?>{
  'byte_len': length,
  'sha256': List<String>.filled(64, digit).join(),
};

Map<String, Object?> _actualSeal(String value) => <String, Object?>{
  'byte_len': utf8.encode(value).length,
  'sha256': crypto.sha256.convert(utf8.encode(value)).toString(),
};

Map<String, Object?> _generation() => <String, Object?>{
  'executable': _seal(171698176, 'a'),
};

Map<String, Object?> _head() => <String, Object?>{
  'store_format': 1,
  'snapshot': _seal(4096, 'b'),
};

String _headJson() => jsonEncode(_head());

Map<String, Object?> _ref(String id, String kind) => <String, Object?>{
  'project_id': _projectId,
  'id': id,
  'expected_kind': kind,
};

Map<String, Object?> _parent(String role, String runtimeClass, String digit) =>
    <String, Object?>{
      'generation': _generation(),
      'source_seal': _seal(400 + role.length, digit),
      'catalog_layer': 'base-game.g1r.scripts',
      'canonical_selector': 'Catalog_${List<String>.filled(64, digit).join()}',
      'runtime_class': runtimeClass,
    };

Map<String, Object?> _input() => <String, Object?>{
  'target': _generation(),
  'module_namespace': _namespace,
  'unique_name': _uniqueName,
  'parent_character_definition': _parent(
    'character_definition',
    'UCharacterDefinition_Human_OM_GRD_Asghan_263',
    '1',
  ),
  'parent_ai_agent_config': _parent(
    'ai_agent_config',
    'UAIAgentConfig_Human_OM_GRD_Asghan_263',
    '2',
  ),
  'parent_spawn_definition': _parent(
    'spawn_definition',
    'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
    '3',
  ),
};

String _source() =>
    '''class UCharacterDefinition_Human_$_uniqueName
    : UCharacterDefinition_Human_OM_GRD_Asghan_263
{
    default m_UniqueName = n"$_uniqueName";
}

class UAIAgentConfig_Human_$_uniqueName
    : UAIAgentConfig_Human_OM_GRD_Asghan_263
{
    default m_CharacterDefinition =
        UCharacterDefinition_Human_$_uniqueName::StaticClass();
}

class USpawnAIAgentDefinition_$_uniqueName
    : USpawnAIAgentDefinition_OM_GRD_Asghan_263
{
    default AIAgentConfigClass =
        UAIAgentConfig_Human_$_uniqueName::StaticClass();
}
''';

Uint8List _u64(int value) =>
    (ByteData(8)..setUint64(0, value, Endian.big)).buffer.asUint8List();

String _inputFingerprint(Map<String, Object?> input) {
  final canonical = utf8.encode(jsonEncode(input));
  final generator = utf8.encode(_generator);
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore-authoring.revision3.npc-draft.input-fingerprint\u0000'),
    )
    ..add(_u64(generator.length))
    ..add(generator)
    ..add(_u64(1))
    ..add(_u64(canonical.length))
    ..add(canonical);
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Map<String, Object?> _diagnostic(
  String code,
  String severity,
  Map<String, Object?> entity,
  String path,
  String message,
) => <String, Object?>{
  'code': code,
  'severity': severity,
  'entity': entity,
  'property_path': path,
  'message': message,
  'blocks_build': true,
};

Map<String, Object?> _plan() {
  final input = _input();
  final inputJson = jsonEncode(input);
  final source = _source();
  final sourceSeal = _actualSeal(source);
  return <String, Object?>{
    'format': 'revision3_npc_source_inspection_plan',
    'schema_revision': 1,
    'scope': 'source_readiness_inspection_only',
    'source_status': 'persisted_and_regenerated_exact',
    'compiler_status': 'not_run',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'spawn_status': 'not_supported',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': _projectId,
      'project_revision': _revision,
      'target': _generation(),
      'canonical_project': _seal(2048, '9'),
    },
    'npc': <String, Object?>{
      'reference': _ref(_npcId, 'npc_draft'),
      'entity_revision': 2,
      'display_name': 'Gate Guard',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': _uniqueName,
      },
      'generator_id': _generator,
      'generator_version': 1,
      'input': input,
      'input_seal': _actualSeal(inputJson),
      'script_module': _ref(_moduleId, 'script_module'),
    },
    'module': <String, Object?>{
      'reference': _ref(_moduleId, 'script_module'),
      'entity_revision': 3,
      'display_name': _namespace,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': _generator,
        'generator_version': 1,
        'owner': _ref(_npcId, 'npc_draft'),
      },
      'persisted_source': sourceSeal,
      'generated': <String, Object?>{
        'generator_id': _generator,
        'generator_version': 1,
        'owner': _ref(_npcId, 'npc_draft'),
        'module_namespace': _namespace,
        'module_relative_path': 'GoreMods/Npcs/GateGuard.as',
        'source': source,
        'source_sha256': sourceSeal['sha256'],
        'input_fingerprint': _inputFingerprint(input),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
    'diagnostics': <Object?>[
      _diagnostic(
        'NPC_COMPILER_NOT_RUN',
        'warning',
        _ref(_moduleId, 'script_module'),
        'payload.data.source',
        'The exact generated NPC source was not submitted to a compiler by this read-only inspection.',
      ),
      _diagnostic(
        'NPC_PRODUCTION_LOWERING_UNAVAILABLE',
        'error',
        _ref(_npcId, 'npc_draft'),
        'payload.data.script_module',
        'Production lowering for revision-3 NPC drafts is unavailable.',
      ),
      _diagnostic(
        'NPC_RUNTIME_RESIDENCE_UNQUALIFIED',
        'error',
        _ref(_npcId, 'npc_draft'),
        'payload.data.script_module',
        'NPC class residence, effective behavior, distinct state, and persistence are runtime-unqualified.',
      ),
      _diagnostic(
        'NPC_SPAWN_UNAVAILABLE',
        'error',
        _ref(_npcId, 'npc_draft'),
        'payload.data.input',
        'No qualified spawn or world-placement mechanism is available for this NPC draft.',
      ),
    ],
  };
}

Map<String, Object?> _response() {
  final planJson = jsonEncode(_plan());
  return <String, Object?>{
    'build_status': 'blocked',
    'compiler_status': 'not_run',
    'head_json': _headJson(),
    'npc_id': _npcId,
    'ok': true,
    'outcome': 'inspection_only',
    'plan_json': planJson,
    'plan_seal': _actualSeal(planJson),
    'project_id': _projectId,
    'project_revision': _revision,
    'project_seal': _seal(2048, '9'),
    'publication_status': 'not_supported',
    'runtime_qualification': 'runtime_unqualified',
    'scope': 'source_readiness_inspection_only',
    'source_status': 'persisted_and_regenerated_exact',
    'spawn_status': 'not_supported',
  };
}

void _replacePlan(
  Map<String, Object?> response,
  void Function(Map<String, Object?>) mutate,
) {
  final plan = (jsonDecode(response['plan_json']! as String) as Map)
      .cast<String, Object?>();
  mutate(plan);
  final planJson = jsonEncode(plan);
  response['plan_json'] = planJson;
  response['plan_seal'] = _actualSeal(planJson);
}

Future<void> _expectMalformed(Map<String, Object?> response) async {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_store_inspect_revision3_npc_source_v1': response,
    },
  );
  await expectLater(
    ModFfi(core).authoringStoreInspectRevision3NpcSourceV1(
      root: _root,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
      npcId: _npcId,
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
  test('handshake contains the sorted NPC inspection command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_inspect_revision3_npc_source_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'sends only the project read request and parses the closed plan',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_inspect_revision3_npc_source_v1': _response(),
        },
      );
      final result = await ModFfi(core)
          .authoringStoreInspectRevision3NpcSourceV1(
            root: _root,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
            npcId: _npcId,
          );

      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': _headJson(),
        'npc_id': _npcId,
        'root': _root,
      });
      expect(core.calls.single.payload, isNot(contains('game_root')));
      expect(core.calls.single.payload, isNot(contains('project_json')));
      expect(result.npcId, _npcId);
      expect(result.plan.npc.input.uniqueName, _uniqueName);
      expect(
        result.plan.module.generated.moduleRelativePath,
        'GoreMods/Npcs/GateGuard.as',
      );
      expect(result.plan.generatedSource, _source());
      expect(result.plan.diagnostics, hasLength(4));
      expect(
        result.plan.diagnostics.first.code,
        AuthoringRevision3NpcInspectionDiagnosticCode.compilerNotRun,
      );
      expect(result.plan.knownParentLabel, isNull);
    },
  );

  test(
    'rejects response authority, identity, seal, and field-order drift',
    () async {
      final mutations = <void Function(Map<String, Object?>)>[
        (value) => value['authority'] = 'build',
        (value) => value['outcome'] = 'ready',
        (value) => value['npc_id'] = _otherId,
        (value) => value['project_revision'] = _revision + 1,
        (value) => value['compiler_status'] = 'passed',
        (value) => value['build_status'] = 'ready',
        (value) => value['runtime_qualification'] = 'qualified',
        (value) => value['spawn_status'] = 'supported',
        (value) => value['plan_seal'] = _seal(1, 'f'),
        (value) {
          final copy = Map<String, Object?>.from(value);
          value
            ..clear()
            ..['outcome'] = copy['outcome']
            ..addAll(copy);
        },
      ];
      for (final mutate in mutations) {
        final value = _response();
        mutate(value);
        await _expectMalformed(value);
      }
    },
  );

  test(
    'rejects forged nested source, refs, diagnostics, and canonical order',
    () async {
      final mutations = <void Function(Map<String, Object?>)>[
        (plan) => plan['scope'] = 'compile_and_spawn',
        (plan) => (plan['npc']! as Map<String, Object?>)['entity_revision'] =
            0x8000000000000000,
        (plan) =>
            (((plan['npc']! as Map<String, Object?>)['reference']!
                    as Map<String, Object?>)['id'] =
                _otherId),
        (plan) =>
            (((plan['module']! as Map<String, Object?>)['generated']!
                    as Map<String, Object?>)['source'] =
                'forged'),
        (plan) =>
            (((plan['module']! as Map<String, Object?>)['generated']!
                    as Map<String, Object?>)['input_fingerprint'] =
                List<String>.filled(64, 'f').join()),
        (plan) => ((plan['diagnostics']! as List<Object?>).removeLast()),
        (plan) =>
            (((plan['diagnostics']! as List<Object?>).first!
                    as Map<String, Object?>)['blocks_build'] =
                false),
        (plan) {
          final provenance = plan['provenance']! as Map<String, Object?>;
          final target = provenance['target'];
          provenance
            ..remove('target')
            ..['target'] = target;
        },
      ];
      for (final mutate in mutations) {
        final value = _response();
        _replacePlan(value, mutate);
        await _expectMalformed(value);
      }
    },
  );

  test('rejects unsafe request values before calling native', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_inspect_revision3_npc_source_v1': _response(),
      },
    );
    final ffi = ModFfi(core);
    final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());
    for (final request
        in <Future<AuthoringRevision3NpcSourceInspectionResult> Function()>[
          () => ffi.authoringStoreInspectRevision3NpcSourceV1(
            root: '$_root\u0000forged',
            expectedHead: head,
            npcId: _npcId,
          ),
          () => ffi.authoringStoreInspectRevision3NpcSourceV1(
            root: _root,
            expectedHead: head,
            npcId: List<String>.filled(32, '0').join(),
          ),
          () => ffi.authoringStoreInspectRevision3NpcSourceV1(
            root: _root,
            expectedHead: head,
            npcId: 'A${_npcId.substring(1)}',
          ),
        ]) {
      await expectLater(
        request(),
        throwsA(anyOf(isA<ArgumentError>(), isA<FormatException>())),
      );
    }
    expect(core.calls, isEmpty);
  });
}
