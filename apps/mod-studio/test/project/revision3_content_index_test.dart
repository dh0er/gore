import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

Map<String, Object?> _target(String sha) => <String, Object?>{
  'executable': <String, Object?>{'byte_len': 123, 'sha256': sha},
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

Map<String, Object?> _fixture() => <String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': 7,
  'project_name': 'Fixture project',
  'project_version': '0.1.0',
  'project_author': 'GORE',
  'target': _target(_sha),
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{'npc_draft': 1, 'script_module': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': _npcId,
      'kind': 'npc_draft',
      'display_name': 'Gate Guard',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_GATE_GUARD',
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': 'GORE_GATE_GUARD',
          'module_namespace': 'PROJECT.NPCS.GATEGUARD',
          'parent_character_definition': 'UCharacterDefinition_Asghan',
          'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
          'parent_spawn_definition': 'USpawnAIAgentDefinition_Asghan',
        },
      },
      'references': <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _moduleId,
      'kind': 'script_module',
      'display_name': 'Gate Guard source',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.logical-npc-clone-draft',
        'generator_version': 1,
        'owner': <String, Object?>{
          'project_id': _projectId,
          'entity_id': _npcId,
          'expected_kind': 'npc_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'module_namespace': 'PROJECT.NPCS.GATEGUARD',
          'module_relative_path': 'Project/Npcs/GateGuard.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _npcId,
          expectedKind: 'npc_draft',
        ),
        _reference(
          role: 'script_owner',
          targetId: _npcId,
          expectedKind: 'npc_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
};

Map<String, Object?> _clone(Map<String, Object?> value) =>
    Map<String, Object?>.from(jsonDecode(jsonEncode(value)) as Map);

void main() {
  test('parses a closed semantic index without generated source', () {
    final index = Revision3ContentIndex.fromJsonObject(_fixture());

    expect(index.projectId, _projectId);
    expect(index.projectRevision, 7);
    expect(index.entities, hasLength(2));
    expect(index.problemCount, 0);
    expect(index.entities.first.kind, Revision3ContentEntityKind.npcDraft);
    expect(index.entities.first.summary.primaryIdentity, 'GORE_GATE_GUARD');
    expect(index.entities.first.matches('asghan'), isTrue);
    expect(index.entities.last.matches('secretgeneratedbody'), isFalse);
  });

  test(
    'recomputes typed-reference resolution instead of trusting native text',
    () {
      final fixture = _clone(_fixture());
      final entities = fixture['entities']! as List<Object?>;
      final npc = entities.first! as Map<String, Object?>;
      final references = npc['references']! as List<Object?>;
      final reference = references.first! as Map<String, Object?>;
      reference['resolution'] = 'missing_entity';

      expect(
        () => Revision3ContentIndex.fromJsonObject(fixture),
        throwsFormatException,
      );
    },
  );

  test('rejects false counts, noncanonical IDs, and unknown fields', () {
    final falseCount = _clone(_fixture());
    (falseCount['entity_counts']! as Map<String, Object?>)['npc_draft'] = 2;
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseCount),
      throwsFormatException,
    );

    final badId = _clone(_fixture());
    final entities = badId['entities']! as List<Object?>;
    (entities.first! as Map<String, Object?>)['id'] = 'A${_npcId.substring(1)}';
    expect(
      () => Revision3ContentIndex.fromJsonObject(badId),
      throwsFormatException,
    );

    final unknown = _clone(_fixture());
    unknown['extra'] = false;
    expect(
      () => Revision3ContentIndex.fromJsonObject(unknown),
      throwsFormatException,
    );
  });

  test('checks asset classification and signed revision boundary', () {
    final fixture = _clone(_fixture());
    fixture['assets'] = <Object?>[
      <String, Object?>{
        'sha256': _sha,
        'byte_len': 10,
        'media_type': 'audio/ogg',
        'class': 'other',
      },
    ];
    expect(
      () => Revision3ContentIndex.fromJsonObject(fixture),
      throwsFormatException,
    );

    final hugeRevision = _clone(_fixture());
    hugeRevision['project_revision'] = 0x8000000000000000;
    expect(
      () => Revision3ContentIndex.fromJsonObject(hugeRevision),
      throwsFormatException,
    );
  });
}
