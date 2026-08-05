import 'dart:collection';
import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';

const revision3NpcFixtureGeneratorId = 'gore-authoring.logical-npc-clone-draft';
const revision3NpcFixtureGeneratorVersion = 1;
const revision3NpcFixtureCatalogLayer = 'base-game.g1r.scripts';
const revision3NpcInspectionProjectId = '00000000000000000000000000000003';
const revision3NpcInspectionNpcId = '00000000000000000000000000000051';
const revision3NpcInspectionModuleId = '00000000000000000000000000000052';
const revision3NpcInspectionUniqueName = 'GORE_InspectionGuard';
const revision3NpcInspectionModuleNamespace = 'GoreMods.Npcs.InspectionGuard';

typedef _Revision3NpcFixtureParent = ({
  int byteLength,
  String sha256,
  String selector,
  String runtimeClass,
});

typedef _Revision3NpcFixtureSelection = ({
  _Revision3NpcFixtureParent characterDefinition,
  _Revision3NpcFixtureParent aiAgentConfig,
  _Revision3NpcFixtureParent spawnDefinition,
});

/// Frozen values emitted by the native pinned Story-catalog projection shared
/// by every currently registered generation.
/// Selector strings are intentionally literal rather than shared with the
/// Dart verifier so an endian/domain regression cannot make both sides pass.
const Map<String, _Revision3NpcFixtureSelection>
_revision3NpcFixtureNativeSelections = {
  'g1r:npc:om_grd_asghan_263': (
    characterDefinition: (
      byteLength: 460,
      sha256:
          '2312e01be5dd91d043b03acbd487f310d47b99107d765ce31ad87aa77eb5723e',
      selector:
          'Catalog_7fd932b1d1c19b30b499e6d66fb2650215781ddfa2741bad9e24ea36ed2fb4a9',
      runtimeClass: 'UCharacterDefinition_Human_OM_GRD_Asghan_263',
    ),
    aiAgentConfig: (
      byteLength: 932,
      sha256:
          'b728be66667b1b220438c40c11d0881eab01f6a7cc9094ea935b90a1da36eae8',
      selector:
          'Catalog_11ffa8c562a1b3ca45b12b0eccf73d860118c128d730f20c74dce024db76a13a',
      runtimeClass: 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
    ),
    spawnDefinition: (
      byteLength: 96033,
      sha256:
          'e49a3a5f8ac2a589f40878f6f248ab8743adefeab07081754f681cb85c36b86b',
      selector:
          'Catalog_37c0a6b074fb11a47996ac550360d5b3ccfe6982234c6bf9e160d15acf53dd10',
      runtimeClass: 'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
    ),
  ),
  'g1r:npc:om_stt_viper_302': (
    characterDefinition: (
      byteLength: 455,
      sha256:
          '1a4c6caad0511154f4622722f38ec5f85cc2e12f500224f90f4e0208614e7c73',
      selector:
          'Catalog_4d7e588741075a66727a1bbd3476e063c8ff534dae7b050e04461ceb80f33a8b',
      runtimeClass: 'UCharacterDefinition_Human_OM_STT_Viper_302',
    ),
    aiAgentConfig: (
      byteLength: 932,
      sha256:
          'dde3f35f70f23a1ae77f0768d7a947fc2fbd9deaac4b3c12a5bad4f35725220b',
      selector:
          'Catalog_20848be7740fdaec4ddeff3e9fbbc96ed77c75971ee87f02bec53a958840661e',
      runtimeClass: 'UAIAgentConfig_Human_OM_STT_Viper_302',
    ),
    spawnDefinition: (
      byteLength: 96033,
      sha256:
          'e49a3a5f8ac2a589f40878f6f248ab8743adefeab07081754f681cb85c36b86b',
      selector:
          'Catalog_47f6afea7b95a3c36aa5e6aaa89d08510c583da6bb8eda3806d519c8c0fc8ab1',
      runtimeClass: 'USpawnAIAgentDefinition_OM_STT_Viper_302',
    ),
  ),
};

/// Frozen test projection of the native revision-3 NPC transaction wire.
///
/// It deliberately starts from arbitrary exact basis bytes and preserves every preexisting field
/// and entity. Only the requested NPC/module pair and project revision are advanced.
final class Revision3NpcFixture {
  Revision3NpcFixture._({
    required this.basisHead,
    required this.basisProjectJson,
    required this.request,
    required this.candidateHead,
    required this.candidateProjectJson,
  });

  factory Revision3NpcFixture.fromBasis({
    required AuthoringWorkingHead basisHead,
    required String basisProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  }) {
    final basis = (jsonDecode(basisProjectJson) as Map).cast<String, Object?>();
    final projectId = basis['project_id']! as String;
    final target = (basis['target']! as Map).cast<String, Object?>();
    final input = revision3NpcFixtureInput(request: request, target: target);
    final entities = SplayTreeMap<String, Object?>.from(
      (basis['entities']! as Map).cast<String, Object?>(),
    );
    entities[request.npcId] = revision3NpcFixtureEntity(
      projectId: projectId,
      request: request,
      input: input,
    );
    entities[request.scriptModuleId] = revision3NpcFixtureModuleEntity(
      projectId: projectId,
      request: request,
      input: input,
    );
    basis['revision'] = request.expectedRevision + 1;
    basis['entities'] = entities;
    final candidateProjectJson = jsonEncode(basis);
    final candidateHead = revision3NpcFixtureHead(candidateProjectJson);
    return Revision3NpcFixture._(
      basisHead: basisHead,
      basisProjectJson: basisProjectJson,
      request: request,
      candidateHead: candidateHead,
      candidateProjectJson: candidateProjectJson,
    );
  }

  final AuthoringWorkingHead basisHead;
  final String basisProjectJson;
  final AuthoringRevision3NpcDraftRequestV1 request;
  final AuthoringWorkingHead candidateHead;
  final String candidateProjectJson;

  Map<String, Object?> response() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basisHead.canonicalJson,
    'head_json': candidateHead.canonicalJson,
    'project_json': candidateProjectJson,
    'revision': request.expectedRevision + 1,
    'npc_id': request.npcId,
    'script_module_id': request.scriptModuleId,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'catalog_authority': 'not_granted',
    'collision_authority': 'not_granted',
    'source_inspection': 'fresh_native_context_required',
    'publication_status': 'not_supported',
  };
}

AuthoringWorkingHead revision3NpcFixtureHead(String projectJson) {
  final bytes = utf8.encode(projectJson);
  return AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{
        'byte_len': bytes.length,
        'sha256': crypto.sha256.convert(bytes).toString(),
      },
    }),
  );
}

Map<String, Object?> revision3NpcFixtureInput({
  required AuthoringRevision3NpcDraftRequestV1 request,
  required Map<String, Object?> target,
}) {
  final selection =
      _revision3NpcFixtureNativeSelections[request.intent.parentCatalogId];
  if (selection == null) {
    throw ArgumentError.value(
      request.intent.parentCatalogId,
      'request.intent.parentCatalogId',
      'fixture has no frozen native selection vector',
    );
  }
  return <String, Object?>{
    'target': target,
    'module_namespace': request.intent.moduleNamespace,
    'unique_name': request.intent.uniqueName,
    'parent_character_definition': _parent(
      target: target,
      evidence: selection.characterDefinition,
    ),
    'parent_ai_agent_config': _parent(
      target: target,
      evidence: selection.aiAgentConfig,
    ),
    'parent_spawn_definition': _parent(
      target: target,
      evidence: selection.spawnDefinition,
    ),
  };
}

Map<String, Object?> revision3NpcFixtureEntity({
  required String projectId,
  required AuthoringRevision3NpcDraftRequestV1 request,
  required Map<String, Object?> input,
}) => <String, Object?>{
  'id': request.npcId,
  'display_name': request.displayName,
  'origin': <String, Object?>{
    'type': 'new',
    'authored_runtime_id': request.intent.uniqueName,
  },
  'revision': 0,
  'payload': <String, Object?>{
    'kind': 'npc_draft',
    'data': <String, Object?>{
      'generator_id': revision3NpcFixtureGeneratorId,
      'generator_version': revision3NpcFixtureGeneratorVersion,
      'input': input,
      'script_module': _typedRef(
        projectId,
        request.scriptModuleId,
        'script_module',
      ),
    },
  },
};

Map<String, Object?> revision3NpcFixtureModuleEntity({
  required String projectId,
  required AuthoringRevision3NpcDraftRequestV1 request,
  required Map<String, Object?> input,
}) {
  final source = revision3NpcFixtureSource(input);
  return <String, Object?>{
    'id': request.scriptModuleId,
    'display_name': request.intent.moduleNamespace,
    'origin': <String, Object?>{
      'type': 'generated',
      'generator_id': revision3NpcFixtureGeneratorId,
      'generator_version': revision3NpcFixtureGeneratorVersion,
      'owner': _typedRef(projectId, request.npcId, 'npc_draft'),
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'script_module',
      'data': <String, Object?>{
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': revision3NpcFixtureGeneratorVersion,
        'owner': _typedRef(projectId, request.npcId, 'npc_draft'),
        'module_namespace': request.intent.moduleNamespace,
        'module_relative_path':
            '${request.intent.moduleNamespace.replaceAll('.', '/')}.as',
        'source': source,
        'source_sha256': crypto.sha256.convert(utf8.encode(source)).toString(),
        'input_fingerprint': revision3NpcFixtureInputFingerprint(input),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  };
}

String revision3NpcFixtureSource(Map<String, Object?> input) {
  final uniqueName = input['unique_name']! as String;
  final character = 'UCharacterDefinition_Human_$uniqueName';
  final agent = 'UAIAgentConfig_Human_$uniqueName';
  final spawn = 'USpawnAIAgentDefinition_$uniqueName';
  final parentCharacter = _runtimeClass(input, 'parent_character_definition');
  final parentAgent = _runtimeClass(input, 'parent_ai_agent_config');
  final parentSpawn = _runtimeClass(input, 'parent_spawn_definition');
  return '''class $character
    : $parentCharacter
{
    default m_UniqueName = n"$uniqueName";
}

class $agent
    : $parentAgent
{
    default m_CharacterDefinition =
        $character::StaticClass();
}

class $spawn
    : $parentSpawn
{
    default AIAgentConfigClass =
        $agent::StaticClass();
}
''';
}

String revision3NpcFixtureInputFingerprint(Map<String, Object?> input) {
  final canonical = utf8.encode(jsonEncode(input));
  final generator = utf8.encode(revision3NpcFixtureGeneratorId);
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore-authoring.revision3.npc-draft.input-fingerprint\u0000'),
    )
    ..add(_uint64(generator.length))
    ..add(generator)
    ..add(_uint64(revision3NpcFixtureGeneratorVersion))
    ..add(_uint64(canonical.length))
    ..add(canonical);
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Map<String, Object?> _parent({
  required Map<String, Object?> target,
  required _Revision3NpcFixtureParent evidence,
}) => <String, Object?>{
  'generation': target,
  'source_seal': <String, Object?>{
    'byte_len': evidence.byteLength,
    'sha256': evidence.sha256,
  },
  'catalog_layer': revision3NpcFixtureCatalogLayer,
  'canonical_selector': evidence.selector,
  'runtime_class': evidence.runtimeClass,
};

Map<String, Object?> _typedRef(String projectId, String id, String kind) =>
    <String, Object?>{'project_id': projectId, 'id': id, 'expected_kind': kind};

String _runtimeClass(Map<String, Object?> input, String field) =>
    ((input[field]! as Map)['runtime_class'])! as String;

Uint8List _uint64(int value) {
  final data = ByteData(8)..setUint64(0, value, Endian.big);
  return data.buffer.asUint8List();
}

/// Fully closed NPC source-inspection result for Session, Controller, and UI
/// tests. Every seal is derived from the supplied exact bytes; optional
/// identity overrides deliberately keep the DTO internally consistent so the
/// next integration layer can prove that it rejects a response-basis drift.
AuthoringRevision3NpcSourceInspectionResult revision3NpcInspectionResult({
  required AuthoringWorkingHead head,
  required String projectJson,
  String? projectId,
  int? projectRevision,
  String npcId = revision3NpcInspectionNpcId,
  String moduleId = revision3NpcInspectionModuleId,
  String displayName = 'Inspection Guard',
  String uniqueName = revision3NpcInspectionUniqueName,
  String moduleNamespace = revision3NpcInspectionModuleNamespace,
  String? projectSealJson,
}) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final responseProjectId = projectId ?? project['project_id']! as String;
  final responseRevision = projectRevision ?? project['revision']! as int;
  final target = (project['target']! as Map).cast<String, Object?>();
  final request = AuthoringRevision3NpcDraftRequestV1.forProject(
    expectedHead: head,
    currentProjectJson: jsonEncode(<String, Object?>{
      ...project,
      'project_id': responseProjectId,
      'revision': responseRevision,
    }),
    npcId: npcId,
    scriptModuleId: moduleId,
    displayName: displayName,
    intent: AuthoringRevision3NpcDraftIntentV1(
      moduleNamespace: moduleNamespace,
      uniqueName: uniqueName,
      parentCatalogId: 'g1r:npc:om_grd_asghan_263',
    ),
  );
  final input = revision3NpcFixtureInput(request: request, target: target);
  final inputJson = jsonEncode(input);
  final source = revision3NpcFixtureSource(input);
  final sourceSeal = _bytesSeal(source);
  final sealedProject = projectSealJson ?? projectJson;
  final canonicalProjectSeal = _bytesSeal(sealedProject);
  final npcRef = _typedRef(responseProjectId, npcId, 'npc_draft');
  final moduleRef = _typedRef(responseProjectId, moduleId, 'script_module');
  final planJson = jsonEncode(<String, Object?>{
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
      'project_id': responseProjectId,
      'project_revision': responseRevision,
      'target': target,
      'canonical_project': canonicalProjectSeal,
    },
    'npc': <String, Object?>{
      'reference': npcRef,
      'entity_revision': 2,
      'display_name': displayName,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': uniqueName,
      },
      'generator_id': revision3NpcFixtureGeneratorId,
      'generator_version': revision3NpcFixtureGeneratorVersion,
      'input': input,
      'input_seal': _bytesSeal(inputJson),
      'script_module': moduleRef,
    },
    'module': <String, Object?>{
      'reference': moduleRef,
      'entity_revision': 3,
      'display_name': moduleNamespace,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': revision3NpcFixtureGeneratorVersion,
        'owner': npcRef,
      },
      'persisted_source': sourceSeal,
      'generated': <String, Object?>{
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': revision3NpcFixtureGeneratorVersion,
        'owner': npcRef,
        'module_namespace': moduleNamespace,
        'module_relative_path': '${moduleNamespace.replaceAll('.', '/')}.as',
        'source': source,
        'source_sha256': sourceSeal['sha256'],
        'input_fingerprint': revision3NpcFixtureInputFingerprint(input),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
    'diagnostics': <Object?>[
      _inspectionDiagnostic(
        code: 'NPC_COMPILER_NOT_RUN',
        severity: 'warning',
        entity: moduleRef,
        propertyPath: 'payload.data.source',
        message:
            'The exact generated NPC source was not submitted to a compiler by this read-only inspection.',
      ),
      _inspectionDiagnostic(
        code: 'NPC_PRODUCTION_LOWERING_UNAVAILABLE',
        severity: 'error',
        entity: npcRef,
        propertyPath: 'payload.data.script_module',
        message:
            'Production lowering for revision-3 NPC drafts is unavailable.',
      ),
      _inspectionDiagnostic(
        code: 'NPC_RUNTIME_RESIDENCE_UNQUALIFIED',
        severity: 'error',
        entity: npcRef,
        propertyPath: 'payload.data.script_module',
        message:
            'NPC class residence, effective behavior, distinct state, and persistence are runtime-unqualified.',
      ),
      _inspectionDiagnostic(
        code: 'NPC_SPAWN_UNAVAILABLE',
        severity: 'error',
        entity: npcRef,
        propertyPath: 'payload.data.input',
        message:
            'No qualified spawn or world-placement mechanism is available for this NPC draft.',
      ),
    ],
  });
  final planSeal = _bytesSeal(planJson);
  return AuthoringRevision3NpcSourceInspectionResult.fromJson(
    <String, Object?>{
      // gore-ffi builds the outer response with json!, whose serde_json map
      // order is lexical when preserve_order is disabled.
      'build_status': 'blocked',
      'compiler_status': 'not_run',
      'head_json': head.canonicalJson,
      'npc_id': npcId,
      'ok': true,
      'outcome': 'inspection_only',
      'plan_json': planJson,
      'plan_seal': planSeal,
      'project_id': responseProjectId,
      'project_revision': responseRevision,
      'project_seal': canonicalProjectSeal,
      'publication_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'scope': 'source_readiness_inspection_only',
      'source_status': 'persisted_and_regenerated_exact',
      'spawn_status': 'not_supported',
    },
    expectedHead: head,
    requestedNpcId: npcId,
  );
}

String revision3NpcInspectionProjectJson({
  String projectId = revision3NpcInspectionProjectId,
  int revision = 7,
  String name = 'NPC inspection fixture',
  int executableByteLength = 171698176,
  String executableSha256 =
      'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': projectId,
  'revision': revision,
  'meta': <String, Object?>{
    'name': name,
    'version': '1.0.0',
    'author': 'NPC inspection tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': executableByteLength,
      'sha256': executableSha256,
    },
  },
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _bytesSeal(String value) {
  final bytes = utf8.encode(value);
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

Map<String, Object?> _inspectionDiagnostic({
  required String code,
  required String severity,
  required Map<String, Object?> entity,
  required String propertyPath,
  required String message,
}) => <String, Object?>{
  'code': code,
  'severity': severity,
  'entity': entity,
  'property_path': propertyPath,
  'message': message,
  'blocks_build': true,
};
