import 'dart:collection';
import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';

import 'revision3_npc_fixture.dart';

const revision3NpcProfileProjectId = '00000000000000000000000000000003';
const revision3NpcProfileNpcId = '00000000000000000000000000000081';
const revision3NpcProfileModuleId = '00000000000000000000000000000082';
const revision3NpcProfileAsghanId = 'g1r:npc:om_grd_asghan_263';
const revision3NpcProfileViperId = 'g1r:npc:om_stt_viper_302';
const revision3NpcProfileAliasId = 'g1r:npc:zz_asghan_alias_263';
const revision3NpcProfileExecutableByteLength = 171698176;
const revision3NpcProfileExecutableSha256 =
    'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5';

final class Revision3NpcProfileTestFixture {
  Revision3NpcProfileTestFixture._({
    required this.projectJson,
    required this.head,
    required this.seed,
    required this.index,
    required this.asghanTriple,
    required this.viperTriple,
  });

  factory Revision3NpcProfileTestFixture.create({int moduleRevision = 0}) {
    final empty = revision3NpcProfileEmptyProjectJson();
    final emptyHead = revision3NpcFixtureHead(empty);
    final asghanRequest = _draftRequest(
      projectJson: empty,
      head: emptyHead,
      parentCatalogId: revision3NpcProfileAsghanId,
    );
    final created = Revision3NpcFixture.fromBasis(
      basisHead: emptyHead,
      basisProjectJson: empty,
      request: asghanRequest,
    );
    var projectJson = created.candidateProjectJson;
    var head = created.candidateHead;
    if (moduleRevision != 0) {
      final project = _map(jsonDecode(projectJson));
      final module = _map(
        _map(project['entities'])[revision3NpcProfileModuleId],
      );
      module['revision'] = moduleRevision;
      projectJson = jsonEncode(project);
      head = revision3NpcFixtureHead(projectJson);
    }
    final asghanInput = _npcInput(projectJson);
    final target = ((jsonDecode(empty) as Map)['target'] as Map)
        .cast<String, Object?>();
    final viperInput = revision3NpcFixtureInput(
      request: _draftRequest(
        projectJson: empty,
        head: emptyHead,
        parentCatalogId: revision3NpcProfileViperId,
      ),
      target: target,
    );
    final asghanTriple = _catalogTriple(asghanInput);
    final viperTriple = _catalogTriple(viperInput);
    final seed = AuthoringRevision3NpcProfileEditSeed.forProject(
      head: head,
      currentProjectJson: projectJson,
      npcId: revision3NpcProfileNpcId,
      expectedNpcRevision: 0,
      expectedScriptModuleId: revision3NpcProfileModuleId,
      expectedScriptModuleRevision: moduleRevision,
      expectedUniqueName: 'GoreManagedGuard',
      expectedModuleNamespace: 'GoreMods.Npcs.ManagedGuard',
      expectedParentCharacterDefinition:
          asghanTriple.characterDefinition.runtimeClass,
      expectedParentAiAgentConfig: asghanTriple.aiAgentConfig.runtimeClass,
      expectedParentSpawnDefinition: asghanTriple.spawnDefinition.runtimeClass,
    );
    return Revision3NpcProfileTestFixture._(
      projectJson: projectJson,
      head: head,
      seed: seed,
      index: _contentIndex(seed),
      asghanTriple: asghanTriple,
      viperTriple: viperTriple,
    );
  }

  final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringRevision3NpcProfileEditSeed seed;
  final Revision3ContentIndex index;
  final Revision3NpcCatalogParentTriple asghanTriple;
  final Revision3NpcCatalogParentTriple viperTriple;

  Revision3ContentEntity get npc => index.entityById(seed.npcId)!;

  Revision3NpcCatalog catalog({
    bool includeAlias = false,
    String storySealDigit = '1',
    String npcSealDigit = '2',
  }) => Revision3NpcCatalog(
    choices: <Revision3NpcCatalogChoice>[
      Revision3NpcCatalogChoice(
        catalogId: revision3NpcProfileAsghanId,
        displayName: 'Asghan',
        parentTriple: asghanTriple,
      ),
      Revision3NpcCatalogChoice(
        catalogId: revision3NpcProfileViperId,
        displayName: 'Viper',
        parentTriple: viperTriple,
      ),
      if (includeAlias)
        Revision3NpcCatalogChoice(
          catalogId: revision3NpcProfileAliasId,
          displayName: 'Asghan alias',
          parentTriple: asghanTriple,
        ),
    ],
    generationExecutableSeal: revision3NpcProfileSeal(
      revision3NpcProfileExecutableByteLength,
      revision3NpcProfileExecutableSha256,
    ),
    storyCatalogSeal: revision3NpcProfileSeal(
      1,
      List<String>.filled(64, storySealDigit).join(),
    ),
    npcCatalogSeal: revision3NpcProfileSeal(
      1,
      List<String>.filled(64, npcSealDigit).join(),
    ),
  );
}

AuthoringDraftContentSeal revision3NpcProfileSeal(
  int byteLength,
  String sha256,
) => AuthoringDraftContentSeal.fromJson(<String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
});

AuthoringRevision3NpcProfileParentTripleExpectation
revision3NpcProfileExpectation(Revision3NpcCatalogParentTriple triple) =>
    AuthoringRevision3NpcProfileParentTripleExpectation(
      characterDefinition: _expectation(triple.characterDefinition),
      aiAgentConfig: _expectation(triple.aiAgentConfig),
      spawnDefinition: _expectation(triple.spawnDefinition),
    );

String revision3NpcProfileEmptyProjectJson() => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': revision3NpcProfileProjectId,
  'revision': 0,
  'meta': <String, Object?>{
    'name': 'R3 NPC profile adapter',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': revision3NpcProfileExecutableByteLength,
      'sha256': revision3NpcProfileExecutableSha256,
    },
  },
  'authoring_locales': <Object?>[],
  'entities': SplayTreeMap<String, Object?>(),
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

AuthoringRevision3NpcDraftRequestV1 _draftRequest({
  required String projectJson,
  required AuthoringWorkingHead head,
  required String parentCatalogId,
}) => AuthoringRevision3NpcDraftRequestV1.forProject(
  expectedHead: head,
  currentProjectJson: projectJson,
  npcId: revision3NpcProfileNpcId,
  scriptModuleId: revision3NpcProfileModuleId,
  displayName: 'Managed Guard',
  intent: AuthoringRevision3NpcDraftIntentV1(
    moduleNamespace: 'GoreMods.Npcs.ManagedGuard',
    uniqueName: 'GoreManagedGuard',
    parentCatalogId: parentCatalogId,
  ),
);

Revision3ContentIndex _contentIndex(
  AuthoringRevision3NpcProfileEditSeed seed,
) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': seed.projectId,
  'project_revision': seed.projectRevision,
  'project_name': 'R3 NPC profile adapter',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': jsonDecode(seed.targetCanonicalJson),
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{'npc_draft': 1, 'script_module': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': seed.npcId,
      'kind': 'npc_draft',
      'display_name': seed.displayName,
      'revision': seed.npcRevision,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': seed.uniqueName,
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': seed.uniqueName,
          'module_namespace': seed.moduleNamespace,
          'parent_character_definition':
              seed.parentCharacterDefinition.runtimeClass,
          'parent_ai_agent_config': seed.parentAiAgentConfig.runtimeClass,
          'parent_spawn_definition': seed.parentSpawnDefinition.runtimeClass,
          'greeting_count': 0,
        },
      },
      'references': <Object?>[
        _reference(
          projectId: seed.projectId,
          role: 'draft_script_module',
          targetId: seed.scriptModuleId,
          expectedKind: 'script_module',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': seed.scriptModuleId,
      'kind': 'script_module',
      'display_name': seed.moduleNamespace,
      'revision': seed.scriptModuleRevision,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': revision3NpcFixtureGeneratorVersion,
        'owner': <String, Object?>{
          'project_id': seed.projectId,
          'entity_id': seed.npcId,
          'expected_kind': 'npc_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': revision3NpcFixtureGeneratorId,
          'generator_version': revision3NpcFixtureGeneratorVersion,
          'module_namespace': seed.moduleNamespace,
          'module_relative_path':
              '${seed.moduleNamespace.replaceAll('.', '/')}.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _reference(
          projectId: seed.projectId,
          role: 'origin_owner',
          targetId: seed.npcId,
          expectedKind: 'npc_draft',
        ),
        _reference(
          projectId: seed.projectId,
          role: 'script_owner',
          targetId: seed.npcId,
          expectedKind: 'npc_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

Map<String, Object?> _reference({
  required String projectId,
  required String role,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': projectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Revision3NpcCatalogParentTriple _catalogTriple(Map<String, Object?> input) =>
    Revision3NpcCatalogParentTriple(
      characterDefinition: _catalogParent(input, 'parent_character_definition'),
      aiAgentConfig: _catalogParent(input, 'parent_ai_agent_config'),
      spawnDefinition: _catalogParent(input, 'parent_spawn_definition'),
    );

Revision3NpcCatalogParentBinding _catalogParent(
  Map<String, Object?> input,
  String field,
) {
  final parent = _map(input[field]);
  return Revision3NpcCatalogParentBinding(
    catalogLayer: parent['catalog_layer']! as String,
    authoringSelector: parent['canonical_selector']! as String,
    runtimeClass: parent['runtime_class']! as String,
    sourceSeal: AuthoringDraftContentSeal.fromJson(_map(parent['source_seal'])),
  );
}

AuthoringRevision3NpcProfileParentExpectation _expectation(
  Revision3NpcCatalogParentBinding parent,
) => AuthoringRevision3NpcProfileParentExpectation(
  catalogLayer: parent.catalogLayer,
  authoringSelector: parent.authoringSelector,
  runtimeClass: parent.runtimeClass,
  sourceSeal: parent.sourceSeal,
);

Map<String, Object?> _npcInput(String projectJson) {
  final project = _map(jsonDecode(projectJson));
  final entities = _map(project['entities']);
  final npc = _map(entities[revision3NpcProfileNpcId]);
  return _map(_map(_map(npc['payload'])['data'])['input']);
}

Map<String, Object?> _map(Object? value) =>
    (value as Map).cast<String, Object?>();
