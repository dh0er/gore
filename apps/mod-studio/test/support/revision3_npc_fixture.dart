import 'dart:collection';
import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';

const revision3NpcFixtureGeneratorId = 'gore-authoring.logical-npc-clone-draft';
const revision3NpcFixtureGeneratorVersion = 1;
const revision3NpcFixtureCatalogLayer = 'base-game.g1r.scripts';

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

/// Frozen values emitted by the native pinned Story-catalog V1 projection.
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
      utf8.encode('gore-authoring.revision2.npc-draft.input-fingerprint\u0000'),
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
