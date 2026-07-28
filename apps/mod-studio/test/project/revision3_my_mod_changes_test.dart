import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_my_mod_changes.dart';

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';

const _npcId = '77777777777777777777777777777777';
const _moduleId = '88888888888888888888888888888888';
const _missingId = '99999999999999999999999999999999';
const _itemId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _stageManifestMediaType =
    'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1';

void main() {
  test('groups a mutually owned generated Quest module', () {
    final index = Revision3QuestOutlineFixture(
      displayName: 'Zulu quest',
    ).contentIndex();

    final projection = Revision3MyModChanges.fromExactCurrent(
      contentIndex: index,
      dataAssetStages: const <AuthoringRevision3DataAssetStage>[],
    );

    expect(projection.projectId, index.projectId);
    expect(projection.projectRevision, index.projectRevision);
    expect(projection.technical, isEmpty);
    expect(projection.changes, hasLength(1));
    final quest = projection.changes.single;
    expect(quest.kind, Revision3MyModContentKind.quest);
    expect(quest.relationship, Revision3MyModRelationship.topLevel);
    expect(quest.children, hasLength(1));
    expect(
      quest.children.single.kind,
      Revision3MyModContentKind.generatedScript,
    );
    expect(
      quest.children.single.relationship,
      Revision3MyModRelationship.generatedScript,
    );
    _expectEveryEntityExactlyOnce(index, projection);
  });

  test('nests the exact NPC dialog, localization, and voice closure', () {
    final index = _npcVoiceIndex();

    final projection = Revision3MyModChanges.fromExactCurrent(
      contentIndex: index,
      dataAssetStages: const <AuthoringRevision3DataAssetStage>[],
    );

    expect(projection.technical, isEmpty);
    expect(projection.changes, hasLength(1));
    final npc = projection.changes.single;
    expect(npc.kind, Revision3MyModContentKind.npc);
    expect(npc.children.map((entry) => entry.kind), <Revision3MyModContentKind>[
      Revision3MyModContentKind.generatedScript,
      Revision3MyModContentKind.dialogLine,
    ]);

    final dialog = npc.children.last;
    expect(dialog.relationship, Revision3MyModRelationship.npcGreeting);
    expect(
      dialog.children.map((entry) => entry.kind),
      <Revision3MyModContentKind>[
        Revision3MyModContentKind.localization,
        Revision3MyModContentKind.voiceSlot,
      ],
    );
    final localization = dialog.children.first;
    expect(
      localization.relationship,
      Revision3MyModRelationship.dialogLocalization,
    );
    final slot = dialog.children.last;
    expect(slot.relationship, Revision3MyModRelationship.dialogVoiceSlot);
    expect(slot.qualifier, 'de');
    expect(slot.children, hasLength(1));
    final take = slot.children.single;
    expect(take.relationship, Revision3MyModRelationship.voiceCandidate);
    expect(take.selected, isTrue);
    _expectEveryEntityExactlyOnce(index, projection);
  });

  test(
    'keeps shared authored content top-level and generated ambiguity Technical',
    () {
      final json = revision3VoiceContentIndexJsonFixture(
        existingSlotGenerated: true,
        duplicateLine: true,
        lineDisplayName: 'Zulu line',
      );
      final entities = _entities(json);
      final duplicate = _entity(entities, revision3VoiceContentDuplicateLineId);
      duplicate['display_name'] = 'alpha line';
      final summary = _map(_map(duplicate['summary'])['data']);
      summary['voice_slot_locales'] = <Object?>['de'];
      _references(duplicate).add(
        _reference(
          role: 'dialog_voice_slot',
          qualifier: 'de',
          targetId: revision3VoiceContentSlotId,
          expectedKind: 'voice_slot',
        ),
      );

      final index = Revision3ContentIndex.fromJsonObject(json);
      final projection = Revision3MyModChanges.fromExactCurrent(
        contentIndex: index,
        dataAssetStages: const <AuthoringRevision3DataAssetStage>[],
      );

      expect(projection.changes.map((entry) => entry.stableId), <String>[
        revision3VoiceContentDuplicateLineId,
        revision3VoiceContentLineId,
        revision3VoiceContentLocalizationId,
      ]);
      expect(
        projection.changes.take(2).map((entry) => entry.children),
        everyElement(isEmpty),
      );
      expect(
        projection.changes.last.kind,
        Revision3MyModContentKind.localization,
      );
      expect(projection.technical, hasLength(1));
      expect(projection.technical.single.stableId, revision3VoiceContentSlotId);
      expect(
        projection.technical.single.technicalReason,
        Revision3MyModTechnicalReason.unprovenGeneratedOwnership,
      );
      _expectEveryEntityExactlyOnce(index, projection);
    },
  );

  test(
    'keeps authored problems author-facing but generated problems Technical',
    () {
      final json = revision3VoiceContentIndexJsonFixture(
        existingSlotGenerated: true,
        existingSlotCandidateCount: 1,
      );
      final entities = _entities(json);
      final dialog = _entity(entities, revision3VoiceContentLineId);
      final localizationReference = _references(
        dialog,
      ).singleWhere((reference) => reference['role'] == 'dialog_localization');
      _map(localizationReference['target'])['entity_id'] = _missingId;
      localizationReference['resolution'] = 'missing_entity';

      final slot = _entity(entities, revision3VoiceContentSlotId);
      final candidate = _references(
        slot,
      ).singleWhere((reference) => reference['role'] == 'voice_candidate');
      _map(candidate['target'])['entity_id'] = _missingId;
      candidate['resolution'] = 'missing_entity';

      final index = Revision3ContentIndex.fromJsonObject(json);
      final projection = Revision3MyModChanges.fromExactCurrent(
        contentIndex: index,
        dataAssetStages: const <AuthoringRevision3DataAssetStage>[],
      );

      final dialogRow = projection.changes.singleWhere(
        (entry) => entry.stableId == revision3VoiceContentLineId,
      );
      expect(dialogRow.kind, Revision3MyModContentKind.dialogLine);
      expect(dialogRow.problemCount, 1);
      expect(dialogRow.technicalReason, isNull);

      final slotRow = projection.technical.single;
      expect(slotRow.stableId, revision3VoiceContentSlotId);
      expect(slotRow.problemCount, 1);
      expect(
        slotRow.technicalReason,
        Revision3MyModTechnicalReason.unresolvedReference,
      );
      _expectEveryEntityExactlyOnce(index, projection);
    },
  );

  test('ItemPatch is its own row and assets never impersonate DataAssets', () {
    final index = _itemPatchIndex(includeStageManifestAsset: true);

    final projection = Revision3MyModChanges.fromExactCurrent(
      contentIndex: index,
      dataAssetStages: const <AuthoringRevision3DataAssetStage>[],
    );

    expect(projection.technical, isEmpty);
    expect(projection.changes, hasLength(1));
    expect(projection.changes.single.kind, Revision3MyModContentKind.itemPatch);
    expect(
      projection.changes.where(
        (entry) => entry.kind == Revision3MyModContentKind.dataAsset,
      ),
      isEmpty,
    );
    _expectEveryEntityExactlyOnce(index, projection);
  });

  test('projects DataAssets only from the exact stage-registry input', () {
    final fixture = revision3DataAssetNativeGoldenFixture();
    final listed = AuthoringRevision3DataAssetStageListResult.fromJson(
      fixture.listResponse(),
      expectedHead: fixture.stagedHead,
    );
    final stage = listed.stages.single;
    final index = _emptyIndex(
      projectId: stage.projectId,
      revision: listed.revision,
      targetSha: stage.projectTargetExecutable.sha256,
      targetByteLength: stage.projectTargetExecutable.byteLength,
    );

    final projection = Revision3MyModChanges.fromExactCurrent(
      contentIndex: index,
      dataAssetStages: listed.stages,
    );

    expect(projection.technical, isEmpty);
    expect(projection.changes, hasLength(1));
    final row = projection.changes.single;
    expect(row.kind, Revision3MyModContentKind.dataAsset);
    expect(row.stableId, stage.targetPath);
    expect(row.entity, isNull);
    expect(identical(row.dataAssetStage, stage), isTrue);

    final otherProject = _emptyIndex(
      projectId: '17171717171717171717171717171717',
      revision: listed.revision,
      targetSha: stage.projectTargetExecutable.sha256,
      targetByteLength: stage.projectTargetExecutable.byteLength,
    );
    expect(
      () => Revision3MyModChanges.fromExactCurrent(
        contentIndex: otherProject,
        dataAssetStages: listed.stages,
      ),
      throwsA(isA<Revision3MyModSnapshotMismatch>()),
    );
  });
}

Revision3ContentIndex _npcVoiceIndex() {
  final json = revision3VoiceContentIndexJsonFixture(
    existingSlotGenerated: true,
    existingSlotCandidateCount: 1,
    existingSlotHasSelectedTake: true,
    existingSlotTargetResolution: 'resolved',
  );
  final entities = _entities(json);
  entities.addAll(<Map<String, Object?>>[
    <String, Object?>{
      'id': _npcId,
      'kind': 'npc_draft',
      'display_name': 'Managed guard',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GoreManagedGuard',
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': 'GoreManagedGuard',
          'module_namespace': 'GoreMods.Npcs.ManagedGuard',
          'parent_character_definition': 'UCharacterDefinition_Asghan',
          'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
          'parent_spawn_definition': 'USpawnDefinition_Asghan',
          'greeting_count': 1,
        },
      },
      'references': <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
        _reference(
          role: 'npc_greeting_line',
          targetId: revision3VoiceContentLineId,
          expectedKind: 'dialog_line',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _moduleId,
      'kind': 'script_module',
      'display_name': 'Managed guard script',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.logical-npc-clone-draft',
        'generator_version': 1,
        'owner': <String, Object?>{
          'project_id': revision3VoiceContentProjectId,
          'entity_id': _npcId,
          'expected_kind': 'npc_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'module_namespace': 'GoreMods.Npcs.ManagedGuard',
          'module_relative_path': 'GoreMods/Npcs/ManagedGuard.as',
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
  ]);
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  json['entity_counts'] = <String, Object?>{
    'localization_entry': 1,
    'dialog_line': 1,
    'voice_slot': 1,
    'voice_take': 1,
    'npc_draft': 1,
    'script_module': 1,
  };
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _itemPatchIndex({
  required bool includeStageManifestAsset,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': revision3VoiceContentProjectId,
  'project_revision': 5,
  'project_name': 'Item fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256': _targetSha,
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{'item_patch': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': _itemId,
      'kind': 'item_patch',
      'display_name': 'Apple',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'vanilla',
        'generation': <String, Object?>{
          'executable': <String, Object?>{
            'byte_len': 171698176,
            'sha256': _targetSha,
          },
        },
        'catalog_layer': 'base-game.g1r.items.v1',
        'canonical_selector': 'UItemDefinition_Apple',
        'source_seal': <String, Object?>{'byte_len': 456, 'sha256': 'c' * 64},
      },
      'summary': <String, Object?>{
        'kind': 'item_patch',
        'data': <String, Object?>{
          'vanilla_class': 'UItemDefinition_Apple',
          'field_count': 1,
          'field_types': <String, Object?>{'m_Value': 'integer'},
          'fields': <String, Object?>{
            'm_Value': <String, Object?>{'type': 'integer', 'data': 5},
          },
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[
    if (includeStageManifestAsset)
      <String, Object?>{
        'sha256': 'd' * 64,
        'byte_len': 123,
        'media_type': _stageManifestMediaType,
        'class': 'data_asset_stage_manifest',
      },
  ],
});

Revision3ContentIndex _emptyIndex({
  required String projectId,
  required int revision,
  required String targetSha,
  required int targetByteLength,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Empty fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': targetByteLength,
      'sha256': targetSha,
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
});

void _expectEveryEntityExactlyOnce(
  Revision3ContentIndex index,
  Revision3MyModChanges projection,
) {
  final projectedIds = <String>[
    for (final root in <Revision3MyModEntry>[
      ...projection.changes,
      ...projection.technical,
    ])
      for (final entry in _walk(root))
        if (entry.entity case final entity?) entity.id,
  ];
  expect(projectedIds, hasLength(index.entities.length));
  expect(projectedIds.toSet(), <String>{
    for (final item in index.entities) item.id,
  });
}

Iterable<Revision3MyModEntry> _walk(Revision3MyModEntry entry) sync* {
  yield entry;
  for (final child in entry.children) {
    yield* _walk(child);
  }
}

List<Map<String, Object?>> _entities(Map<String, Object?> index) {
  final entities = (index['entities']! as List)
      .map((value) => _map(value))
      .toList(growable: true);
  index['entities'] = entities;
  return entities;
}

Map<String, Object?> _entity(List<Map<String, Object?>> entities, String id) =>
    entities.singleWhere((entity) => entity['id'] == id);

List<Map<String, Object?>> _references(Map<String, Object?> entity) {
  final references = (entity['references']! as List)
      .map((value) => _map(value))
      .toList(growable: true);
  entity['references'] = references;
  return references;
}

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Map<String, Object?> _map(Object? value) =>
    (value! as Map).cast<String, Object?>();
