import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_npc_draft_setup.dart';

const _projectId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _foreignProjectId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _localizationA = '11111111111111111111111111111111';
const _lineA = '22222222222222222222222222222222';
const _localizationB = '33333333333333333333333333333333';
const _lineB = '44444444444444444444444444444444';
const _slotA = '55555555555555555555555555555555';
const _takeA = '66666666666666666666666666666666';
const _takeB = '77777777777777777777777777777777';
const _npcId = '88888888888888888888888888888888';
const _moduleId = '99999999999999999999999999999999';
const _targetSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';

void main() {
  test('N+1 with zero greeting links is a partial persisted setup', () {
    final index = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(projectRevision: 11),
    );
    final npc = index.entityById(_npcId)!;

    final setup = Revision3NpcDraftSetup.fromIndex(index: index, npc: npc);

    expect(setup.projectId, _projectId);
    expect(setup.projectRevision, 11);
    expect(setup.npcId, _npcId);
    expect(setup.npcRevision, 1);
    expect(setup.characterDetailsComplete, isTrue);
    expect(setup.firstGreetingComplete, isFalse);
    expect(setup.greetingLinkCount, 0);
    expect(setup.firstGreetingLineId, isNull);
    expect(setup.firstGreetingLineRevision, isNull);
    expect(setup.firstGreetingDetailsAvailable, isFalse);
    expect(setup.firstGreetingTextLanguageCount, 0);
    expect(setup.firstGreetingVoiceTakeCount, 0);
    expect(setup.firstGreetingSelectedVoiceTakeCount, 0);
    expect(
      setup.complete(Revision3NpcDraftSetupStepKind.characterDetails),
      isTrue,
    );
    expect(
      setup.complete(Revision3NpcDraftSetupStepKind.firstGreeting),
      isFalse,
    );
    expect(setup.draftSetupComplete, isFalse);
    expect(setup.recommendedStep, Revision3NpcDraftSetupStepKind.firstGreeting);
  });

  test('N+2 with one greeting link is complete with exact first context', () {
    final index = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(projectRevision: 12, greetingLineIds: const [_lineA]),
    );

    final setup = Revision3NpcDraftSetup.fromIndex(
      index: index,
      npc: index.entityById(_npcId)!,
    );

    expect(setup.characterDetailsComplete, isTrue);
    expect(setup.firstGreetingComplete, isTrue);
    expect(setup.greetingLinkCount, 1);
    expect(setup.firstGreetingLineId, _lineA);
    expect(setup.firstGreetingLineRevision, 7);
    expect(setup.firstGreetingDetailsAvailable, isTrue);
    expect(setup.firstGreetingTextLanguageCount, 2);
    expect(setup.firstGreetingVoiceTakeCount, 2);
    expect(setup.firstGreetingSelectedVoiceTakeCount, 1);
    expect(setup.draftSetupComplete, isTrue);
    expect(setup.recommendedStep, Revision3NpcDraftSetupStepKind.firstGreeting);
  });

  test('multiple greeting links retain the authored first line', () {
    final index = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(
        projectRevision: 13,
        greetingLineIds: const [_lineB, _lineA],
      ),
    );

    final setup = Revision3NpcDraftSetup.fromIndex(
      index: index,
      npc: index.entityById(_npcId)!,
    );

    expect(setup.greetingLinkCount, 2);
    expect(setup.firstGreetingLineId, _lineB);
    expect(setup.firstGreetingLineRevision, 3);
    expect(setup.firstGreetingDetailsAvailable, isTrue);
    expect(setup.firstGreetingTextLanguageCount, 1);
    expect(setup.firstGreetingVoiceTakeCount, 0);
    expect(setup.firstGreetingSelectedVoiceTakeCount, 0);
    expect(setup.firstGreetingComplete, isTrue);
  });

  test('foreign project, wrong kind, and copied projection fail closed', () {
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    final foreignIndex = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(projectId: _foreignProjectId),
    );
    final copiedIndex = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(),
    );

    for (final entity in <Revision3ContentEntity>[
      foreignIndex.entityById(_npcId)!,
      index.entityById(_lineA)!,
      copiedIndex.entityById(_npcId)!,
    ]) {
      expect(
        () => Revision3NpcDraftSetup.fromIndex(index: index, npc: entity),
        throwsA(isA<Revision3NpcDraftSetupStaleCheckpointException>()),
      );
    }
  });

  test('Character details reject a blank structured fact', () {
    final raw = _contentIndexJson();
    final npc = (raw['entities']! as List<Object?>)
        .cast<Map<String, Object?>>()
        .singleWhere((entity) => entity['id'] == _npcId);
    npc['display_name'] = 'A friendly presentation label';
    final summary = npc['summary']! as Map<String, Object?>;
    final data = summary['data']! as Map<String, Object?>;
    data['parent_spawn_definition'] = '   ';
    final index = Revision3ContentIndex.fromJsonObject(raw);

    final setup = Revision3NpcDraftSetup.fromIndex(
      index: index,
      npc: index.entityById(_npcId)!,
    );

    expect(setup.characterDetailsComplete, isFalse);
    expect(
      setup.recommendedStep,
      Revision3NpcDraftSetupStepKind.characterDetails,
    );
  });

  test('Character details reject a blank visible name', () {
    final raw = _contentIndexJson();
    final npc = (raw['entities']! as List<Object?>)
        .cast<Map<String, Object?>>()
        .singleWhere((entity) => entity['id'] == _npcId);
    npc['display_name'] = '   ';
    final index = Revision3ContentIndex.fromJsonObject(raw);

    final setup = Revision3NpcDraftSetup.fromIndex(
      index: index,
      npc: index.entityById(_npcId)!,
    );

    expect(setup.characterDetailsComplete, isFalse);
    expect(
      setup.recommendedStep,
      Revision3NpcDraftSetupStepKind.characterDetails,
    );
  });

  test('unresolved greeting details retain the proven authored link', () {
    final raw = _contentIndexJson(greetingLineIds: const [_lineA]);
    final entities = raw['entities']! as List<Object?>;
    entities.removeWhere(
      (entity) => (entity! as Map<String, Object?>)['id'] == _localizationA,
    );
    final counts = raw['entity_counts']! as Map<String, Object?>;
    counts['localization_entry'] = 1;
    final line = entities.cast<Map<String, Object?>>().singleWhere(
      (entity) => entity['id'] == _lineA,
    );
    final localizationReference = (line['references']! as List<Object?>)
        .cast<Map<String, Object?>>()
        .singleWhere((reference) => reference['role'] == 'dialog_localization');
    final target = localizationReference['target']! as Map<String, Object?>;
    target['entity_id'] = 'cccccccccccccccccccccccccccccccc';
    localizationReference['resolution'] = 'missing_entity';
    final index = Revision3ContentIndex.fromJsonObject(raw);

    final setup = Revision3NpcDraftSetup.fromIndex(
      index: index,
      npc: index.entityById(_npcId)!,
    );

    expect(setup.firstGreetingComplete, isTrue);
    expect(setup.greetingLinkCount, 1);
    expect(setup.firstGreetingLineId, _lineA);
    expect(setup.firstGreetingLineRevision, 7);
    expect(setup.firstGreetingDetailsAvailable, isFalse);
    expect(setup.firstGreetingTextLanguageCount, 0);
    expect(setup.firstGreetingVoiceTakeCount, 0);
    expect(setup.firstGreetingSelectedVoiceTakeCount, 0);
  });
}

Map<String, Object?> _contentIndexJson({
  String projectId = _projectId,
  int projectRevision = 10,
  List<String> greetingLineIds = const <String>[],
}) => <String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': projectRevision,
  'project_name': 'NPC Draft setup fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 99, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{
    'localization_entry': 2,
    'dialog_line': 2,
    'voice_slot': 1,
    'voice_take': 2,
    'npc_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    _entity(
      id: _localizationA,
      kind: 'localization_entry',
      displayName: 'Gate welcome text',
      revision: 5,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_GATE_WELCOME',
        'locales': <Object?>['de', 'en'],
      },
    ),
    _entity(
      id: _lineA,
      kind: 'dialog_line',
      displayName: 'Gate welcome',
      revision: 7,
      summaryData: <String, Object?>{
        'speaker_hint': 'Guard',
        'voice_slot_locales': <Object?>['de'],
      },
      references: <Object?>[
        _reference(
          projectId: projectId,
          role: 'dialog_localization',
          targetId: _localizationA,
          expectedKind: 'localization_entry',
        ),
        _reference(
          projectId: projectId,
          role: 'dialog_voice_slot',
          qualifier: 'de',
          targetId: _slotA,
          expectedKind: 'voice_slot',
        ),
      ],
    ),
    _entity(
      id: _localizationB,
      kind: 'localization_entry',
      displayName: 'Camp warning text',
      revision: 2,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_CAMP_WARNING',
        'locales': <Object?>['de'],
      },
    ),
    _entity(
      id: _lineB,
      kind: 'dialog_line',
      displayName: 'Camp warning',
      revision: 3,
      summaryData: <String, Object?>{
        'speaker_hint': 'Guard',
        'voice_slot_locales': <Object?>[],
      },
      references: <Object?>[
        _reference(
          projectId: projectId,
          role: 'dialog_localization',
          targetId: _localizationB,
          expectedKind: 'localization_entry',
        ),
      ],
    ),
    _entity(
      id: _slotA,
      kind: 'voice_slot',
      displayName: 'German Voice',
      revision: 4,
      origin: _generatedOrigin(
        projectId: projectId,
        ownerId: _lineA,
        ownerKind: 'dialog_line',
        generatorId: 'gore-authoring.dialog-voice-slot',
      ),
      summaryData: <String, Object?>{
        'locale': 'de',
        'target_resolution': 'resolved',
        'candidate_count': 2,
        'has_selected_take': true,
      },
      references: <Object?>[
        _reference(
          projectId: projectId,
          role: 'origin_owner',
          targetId: _lineA,
          expectedKind: 'dialog_line',
        ),
        _reference(
          projectId: projectId,
          role: 'voice_candidate',
          targetId: _takeA,
          expectedKind: 'voice_take',
        ),
        _reference(
          projectId: projectId,
          role: 'voice_candidate',
          targetId: _takeB,
          expectedKind: 'voice_take',
        ),
        _reference(
          projectId: projectId,
          role: 'voice_selected',
          targetId: _takeB,
          expectedKind: 'voice_take',
        ),
      ],
    ),
    _voiceTake(id: _takeA, locale: 'de', status: 'recorded'),
    _voiceTake(id: _takeB, locale: 'de', status: 'approved'),
    _entity(
      id: _npcId,
      kind: 'npc_draft',
      displayName: 'Managed Guard',
      revision: 1,
      origin: <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GoreManagedGuard',
      },
      summaryData: <String, Object?>{
        'unique_name': 'GoreManagedGuard',
        'module_namespace': 'GoreMods.Npcs.ManagedGuard',
        'parent_character_definition': 'UCharacterDefinition_Asghan',
        'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
        'parent_spawn_definition': 'USpawnDefinition_Asghan',
        'greeting_count': greetingLineIds.length,
      },
      references: <Object?>[
        _reference(
          projectId: projectId,
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
        for (final lineId in greetingLineIds)
          _reference(
            projectId: projectId,
            role: 'npc_greeting_line',
            targetId: lineId,
            expectedKind: 'dialog_line',
          ),
      ],
    ),
    _entity(
      id: _moduleId,
      kind: 'script_module',
      displayName: 'Managed Guard script',
      revision: 6,
      origin: _generatedOrigin(
        projectId: projectId,
        ownerId: _npcId,
        ownerKind: 'npc_draft',
        generatorId: 'gore-authoring.logical-npc-clone-draft',
      ),
      summaryData: <String, Object?>{
        'generator_id': 'gore-authoring.logical-npc-clone-draft',
        'generator_version': 1,
        'module_namespace': 'GoreMods.Npcs.ManagedGuard',
        'module_relative_path': 'GoreMods/Npcs/ManagedGuard.as',
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
      references: <Object?>[
        _reference(
          projectId: projectId,
          role: 'origin_owner',
          targetId: _npcId,
          expectedKind: 'npc_draft',
        ),
        _reference(
          projectId: projectId,
          role: 'script_owner',
          targetId: _npcId,
          expectedKind: 'npc_draft',
        ),
      ],
    ),
  ],
  'assets': <Object?>[],
};

Map<String, Object?> _voiceTake({
  required String id,
  required String locale,
  required String status,
}) => _entity(
  id: id,
  kind: 'voice_take',
  displayName: '$locale Voice take',
  revision: 1,
  summaryData: <String, Object?>{
    'locale': locale,
    'status': status,
    'codec': 'vorbis',
    'channels': 1,
    'sample_rate': 44100,
  },
);

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summaryData,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
}) => <String, Object?>{
  'id': id,
  'kind': kind,
  'display_name': displayName,
  'revision': revision,
  'origin':
      origin ??
      <String, Object?>{'type': 'new', 'authored_runtime_id': 'AUTHORED_$kind'},
  'summary': <String, Object?>{'kind': kind, 'data': summaryData},
  'references': references,
  'asset_references': <Object?>[],
};

Map<String, Object?> _generatedOrigin({
  required String projectId,
  required String ownerId,
  required String ownerKind,
  required String generatorId,
}) => <String, Object?>{
  'type': 'generated',
  'generator_id': generatorId,
  'generator_version': 1,
  'owner': <String, Object?>{
    'project_id': projectId,
    'entity_id': ownerId,
    'expected_kind': ownerKind,
  },
};

Map<String, Object?> _reference({
  required String projectId,
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': projectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
