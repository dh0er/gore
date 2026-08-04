import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _itemPatchId = '44444444444444444444444444444444';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _assetSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _questCollisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';

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
          'greeting_count': 0,
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

Map<String, Object?> _questFixture() => <String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': 7,
  'project_name': 'Quest fixture project',
  'project_version': '0.1.0',
  'project_author': 'GORE',
  'target': _target(_sha),
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{'quest_draft': 1, 'script_module': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': _npcId,
      'kind': 'quest_draft',
      'display_name': 'Guard duty',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_GUARD_DUTY',
      },
      'summary': <String, Object?>{
        'kind': 'quest_draft',
        'data': <String, Object?>{
          'technical_id': 'GORE_GUARD_DUTY',
          'title': 'Guard duty',
          'objective_title': 'Speak to the guard',
          'objective_slots': <Object?>[1],
          'transcript_count': 0,
          'module_namespace': 'PROJECT.QUESTS.GUARDDUTY',
          'parent_runtime_class': 'UQuestDefinition_Base',
          'giver_runtime_unique_name': 'GORE_GATE_GUARD',
        },
      },
      'references': <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
      ],
      'asset_references': <Object?>[
        <String, Object?>{
          'role': 'quest_collision_artifact',
          'sha256': _assetSha,
          'byte_len': 4096,
          'logical_name': null,
          'expected_media_type': _questCollisionMediaType,
          'resolution': 'resolved',
        },
      ],
    },
    <String, Object?>{
      'id': _moduleId,
      'kind': 'script_module',
      'display_name': 'Guard duty source',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': _projectId,
          'entity_id': _npcId,
          'expected_kind': 'quest_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.QUESTS.GUARDDUTY',
          'module_relative_path': 'Project/Quests/GuardDuty.as',
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
          expectedKind: 'quest_draft',
        ),
        _reference(
          role: 'script_owner',
          targetId: _npcId,
          expectedKind: 'quest_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[
    <String, Object?>{
      'sha256': _assetSha,
      'byte_len': 4096,
      'media_type': _questCollisionMediaType,
      'class': 'quest_collision_artifact',
    },
  ],
};

Map<String, Object?> _fixtureWithItemPatch() {
  final fixture = _fixture();
  (fixture['entity_counts']! as Map<String, Object?>)['item_patch'] = 1;
  (fixture['entities']! as List<Object?>).add(<String, Object?>{
    'id': _itemPatchId,
    'kind': 'item_patch',
    'display_name': 'Apple',
    'revision': 2,
    'origin': <String, Object?>{
      'type': 'vanilla',
      'generation': _target(_sha),
      'catalog_layer': 'base-game.g1r.items.v1',
      'canonical_selector': 'UItemDefinition_Apple',
      'source_seal': <String, Object?>{'byte_len': 456, 'sha256': 'c' * 64},
    },
    'summary': <String, Object?>{
      'kind': 'item_patch',
      'data': <String, Object?>{
        'vanilla_class': 'UItemDefinition_Apple',
        'field_count': 5,
        'field_types': <String, Object?>{
          'm_Enabled': 'boolean',
          'm_Kind': 'enum',
          'm_Name': 'string',
          'm_Value': 'integer',
          'm_Weight': 'float',
        },
        'fields': <String, Object?>{
          'm_Enabled': <String, Object?>{'type': 'boolean', 'data': true},
          'm_Kind': <String, Object?>{
            'type': 'enum',
            'data': <String, Object?>{
              'enum_type': 'EItem::Kind',
              'backing': -3,
            },
          },
          'm_Name': <String, Object?>{'type': 'string', 'data': 'Apple'},
          'm_Value': <String, Object?>{'type': 'integer', 'data': 5},
          'm_Weight': <String, Object?>{'type': 'float', 'data': 0.25},
        },
      },
    },
    'references': <Object?>[],
    'asset_references': <Object?>[],
  });
  return fixture;
}

Map<String, Object?> _clone(Map<String, Object?> value) =>
    Map<String, Object?>.from(jsonDecode(jsonEncode(value)) as Map);

void main() {
  test('single-objective Quest fixture omits the empty extension field', () {
    final index = Revision3QuestOutlineFixture(
      objectiveTitles: const <String>['Only objective'],
    ).contentIndex();

    expect(index.entities, hasLength(2));
  });

  test('accepts only exact current native NPC generator ownership', () {
    final index = Revision3ContentIndex.fromJsonObject(_fixture());
    expect(
      index.entityById(_npcId)?.summary.npcDraft?.uniqueName,
      'GORE_GATE_GUARD',
    );

    final oldGenerator = _clone(_fixture());
    final oldModule =
        (oldGenerator['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (oldModule['origin']! as Map<String, Object?>)['generator_version'] = 0;
    final oldSummary = oldModule['summary']! as Map<String, Object?>;
    (oldSummary['data']! as Map<String, Object?>)['generator_version'] = 0;
    expect(
      () => Revision3ContentIndex.fromJsonObject(oldGenerator),
      throwsFormatException,
    );

    final missingOriginOwner = _clone(_fixture());
    final missingOriginModule =
        (missingOriginOwner['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (missingOriginModule['references']! as List<Object?>).removeAt(0);
    expect(
      () => Revision3ContentIndex.fromJsonObject(missingOriginOwner),
      throwsFormatException,
    );

    final missingScriptOwner = _clone(_fixture());
    final missingScriptModule =
        (missingScriptOwner['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (missingScriptModule['references']! as List<Object?>).removeLast();
    expect(
      () => Revision3ContentIndex.fromJsonObject(missingScriptOwner),
      throwsFormatException,
    );

    final falseNpcOrigin = _clone(_fixture());
    final falseNpc =
        (falseNpcOrigin['entities']! as List<Object?>).first!
            as Map<String, Object?>;
    (falseNpc['origin']! as Map<String, Object?>)['authored_runtime_id'] =
        'GORE_ANOTHER_GUARD';
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseNpcOrigin),
      throwsFormatException,
    );

    final falseNpcAsset = _clone(_fixture());
    final assetNpc =
        (falseNpcAsset['entities']! as List<Object?>).first!
            as Map<String, Object?>;
    assetNpc['asset_references'] = <Object?>[
      <String, Object?>{
        'role': 'voice_audio',
        'sha256': _assetSha,
        'byte_len': 1024,
        'logical_name': 'forged_npc_audio.ogg',
        'expected_media_type': 'audio/ogg',
        'resolution': 'resolved',
      },
    ];
    falseNpcAsset['assets'] = <Object?>[
      <String, Object?>{
        'sha256': _assetSha,
        'byte_len': 1024,
        'media_type': 'audio/ogg',
        'class': 'voice_audio',
      },
    ];
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseNpcAsset),
      throwsFormatException,
    );
  });

  test('accepts the current native Quest projection contract', () {
    final index = Revision3ContentIndex.fromJsonObject(_questFixture());

    final quest = index.entityById(_npcId)!;
    expect(quest.summary.questDraft?.transcriptCount, 0);
    expect(
      quest.assetReferences.single.expectedMediaType,
      _questCollisionMediaType,
    );
    expect(
      index.entityById(_moduleId)?.summary.scriptModule?.generatorVersion,
      4,
    );
  });

  test('rejects non-current Quest projection facts', () {
    final missingTranscriptCount = _clone(_questFixture());
    final missingCountQuest =
        (missingTranscriptCount['entities']! as List<Object?>).first!
            as Map<String, Object?>;
    final missingCountSummary =
        missingCountQuest['summary']! as Map<String, Object?>;
    (missingCountSummary['data']! as Map<String, Object?>).remove(
      'transcript_count',
    );
    expect(
      () => Revision3ContentIndex.fromJsonObject(missingTranscriptCount),
      throwsFormatException,
    );

    final oldCollisionMedia = _clone(_questFixture());
    final oldMediaQuest =
        (oldCollisionMedia['entities']! as List<Object?>).first!
            as Map<String, Object?>;
    final oldMediaReference =
        (oldMediaQuest['asset_references']! as List<Object?>).single!
            as Map<String, Object?>;
    const v1 = 'application/vnd.gore.quest-collision-capability+json;version=1';
    oldMediaReference['expected_media_type'] = v1;
    final oldMediaAsset =
        (oldCollisionMedia['assets']! as List<Object?>).single!
            as Map<String, Object?>;
    oldMediaAsset['media_type'] = v1;
    oldMediaAsset['class'] = 'other';
    expect(
      () => Revision3ContentIndex.fromJsonObject(oldCollisionMedia),
      throwsFormatException,
    );

    final oldGenerator = _clone(_questFixture());
    final oldModule =
        (oldGenerator['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (oldModule['origin']! as Map<String, Object?>)['generator_version'] = 3;
    final oldSummary = oldModule['summary']! as Map<String, Object?>;
    (oldSummary['data']! as Map<String, Object?>)['generator_version'] = 3;
    expect(
      () => Revision3ContentIndex.fromJsonObject(oldGenerator),
      throwsFormatException,
    );

    final missingScriptOwner = _clone(_questFixture());
    final missingOwnerModule =
        (missingScriptOwner['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (missingOwnerModule['references']! as List<Object?>).removeLast();
    expect(
      () => Revision3ContentIndex.fromJsonObject(missingScriptOwner),
      throwsFormatException,
    );
  });

  test('retains independently checked typed ItemPatch facts', () {
    final index = Revision3ContentIndex.fromJsonObject(_fixtureWithItemPatch());

    final entity = index.entityById(_itemPatchId)!;
    expect(entity.kind, Revision3ContentEntityKind.itemPatch);
    expect(entity.origin.label, 'UItemDefinition_Apple');
    expect(entity.origin.catalogLayer, 'base-game.g1r.items.v1');
    expect(entity.origin.sourceSeal?.byteLength, 456);
    expect(entity.origin.generationExecutable?.sha256, _sha);
    final item = entity.summary.itemPatch!;
    expect(item.vanillaClass, 'UItemDefinition_Apple');
    expect(item.fields.keys, <String>[
      'm_Enabled',
      'm_Kind',
      'm_Name',
      'm_Value',
      'm_Weight',
    ]);
    expect(item.fields['m_Enabled']!.booleanValue, isTrue);
    expect(item.fields['m_Kind']!.enumValue!.enumType, 'EItem::Kind');
    expect(item.fields['m_Kind']!.enumValue!.backing, -3);
    expect(item.fields['m_Name']!.stringValue, 'Apple');
    expect(item.fields['m_Value']!.integerValue, 5);
    expect(item.fields['m_Weight']!.floatValue, 0.25);
    expect(entity.matches('m_weight'), isTrue);
    expect(
      () => item.fields['another'] = item.fields['m_Value']!,
      throwsUnsupportedError,
    );
  });

  test('rejects false or noncanonical ItemPatch projections', () {
    final falseType = _fixtureWithItemPatch();
    final entity =
        (falseType['entities']! as List<Object?>).last! as Map<String, Object?>;
    final summary = entity['summary']! as Map<String, Object?>;
    final data = summary['data']! as Map<String, Object?>;
    final types = data['field_types']! as Map<String, Object?>;
    types['m_Value'] = 'float';
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseType),
      throwsFormatException,
    );

    final noncanonicalOrder = _fixtureWithItemPatch();
    final reorderedEntity =
        (noncanonicalOrder['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    final reorderedSummary =
        reorderedEntity['summary']! as Map<String, Object?>;
    final reorderedData = reorderedSummary['data']! as Map<String, Object?>;
    final fields = reorderedData['fields']! as Map<String, Object?>;
    final enabled = fields.remove('m_Enabled');
    fields['m_Enabled'] = enabled;
    expect(
      () => Revision3ContentIndex.fromJsonObject(noncanonicalOrder),
      throwsFormatException,
    );

    final falseOrigin = _fixtureWithItemPatch();
    final falseOriginEntity =
        (falseOrigin['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    final origin = falseOriginEntity['origin']! as Map<String, Object?>;
    origin['canonical_selector'] = 'UItemDefinition_Bread';
    expect(
      () => Revision3ContentIndex.fromJsonObject(falseOrigin),
      throwsFormatException,
    );

    final integerFloat = _fixtureWithItemPatch();
    final integerFloatEntity =
        (integerFloat['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    final integerFloatSummary =
        integerFloatEntity['summary']! as Map<String, Object?>;
    final integerFloatData =
        integerFloatSummary['data']! as Map<String, Object?>;
    final integerFloatFields =
        integerFloatData['fields']! as Map<String, Object?>;
    (integerFloatFields['m_Weight']! as Map<String, Object?>)['data'] = 1;
    expect(
      () => Revision3ContentIndex.fromJsonObject(integerFloat),
      throwsFormatException,
    );

    final duplicateTarget = _fixtureWithItemPatch();
    final duplicateEntities = duplicateTarget['entities']! as List<Object?>;
    final duplicate = _clone(
      (duplicateEntities.last! as Map).cast<String, Object?>(),
    );
    duplicate['id'] = '55555555555555555555555555555555';
    duplicateEntities.add(duplicate);
    (duplicateTarget['entity_counts']! as Map<String, Object?>)['item_patch'] =
        2;
    expect(
      () => Revision3ContentIndex.fromJsonObject(duplicateTarget),
      throwsFormatException,
    );

    final badCatalogLayer = _fixtureWithItemPatch();
    final badCatalogEntity =
        (badCatalogLayer['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    (badCatalogEntity['origin']! as Map<String, Object?>)['catalog_layer'] =
        ' base-game.g1r.items.v1';
    expect(
      () => Revision3ContentIndex.fromJsonObject(badCatalogLayer),
      throwsFormatException,
    );

    final badDisplayName = _fixtureWithItemPatch();
    final badDisplayEntity =
        (badDisplayName['entities']! as List<Object?>).last!
            as Map<String, Object?>;
    badDisplayEntity['display_name'] = 'Apple\u0000';
    expect(
      () => Revision3ContentIndex.fromJsonObject(badDisplayName),
      throwsFormatException,
    );
  });

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

  test('derives immutable entity backlinks from validated references', () {
    final index = Revision3ContentIndex.fromJsonObject(_fixture());

    expect(index.entityById(_npcId)?.displayName, 'Gate Guard');
    expect(
      index.entityById(_moduleId)?.kind,
      Revision3ContentEntityKind.scriptModule,
    );
    expect(index.entityById('ffffffffffffffffffffffffffffffff'), isNull);

    final moduleBacklinks = index.backlinksToEntity(_moduleId);
    expect(moduleBacklinks, hasLength(1));
    expect(moduleBacklinks.single.source.id, _npcId);
    expect(moduleBacklinks.single.reference.role, 'draft_script_module');

    final npcBacklinks = index.backlinksToEntity(_npcId);
    expect(npcBacklinks, hasLength(2));
    expect(npcBacklinks.map((backlink) => backlink.reference.role), <String>[
      'origin_owner',
      'script_owner',
    ]);
    expect(
      () => moduleBacklinks.add(moduleBacklinks.single),
      throwsUnsupportedError,
    );
    expect(index.backlinksToAsset(_sha), isEmpty);
  });

  test('derives immutable asset backlinks from validated references', () {
    final index = Revision3ContentIndex.fromJsonObject(_questFixture());
    expect(index.assetBySha256(_assetSha)?.byteLength, 4096);
    expect(index.assetBySha256(_sha), isNull);

    final backlinks = index.backlinksToAsset(_assetSha);
    expect(backlinks, hasLength(1));
    expect(backlinks.single.source.id, _npcId);
    expect(backlinks.single.reference.role, 'quest_collision_artifact');
    expect(backlinks.single.reference.logicalName, isNull);
    expect(() => backlinks.add(backlinks.single), throwsUnsupportedError);
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
