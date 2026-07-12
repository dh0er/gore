import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_catalog_adapter.dart';
import 'package:gore_mod/story/domain/story_draft_requests.dart';

const _catalogJson = '{"format":"story_catalog"}';
const _asghanId = 'g1r:npc:om_grd_asghan_263';
const _viperId = 'g1r:npc:om_stt_viper_302';

void main() {
  late StoryCatalogAdapter adapter;

  setUp(() async {
    adapter = StoryCatalogAdapter.fromSelections(await _selections());
  });

  test('Asghan and Viper choices map exact qualified parent objects', () {
    expect(adapter.npcChoices.map((choice) => choice.catalogId), <String>[
      _asghanId,
      _viperId,
    ]);
    expect(adapter.npcChoices.map((choice) => choice.displayName), <String>[
      'Asghan',
      'Viper',
    ]);
    expect(
      adapter.npcChoices,
      everyElement(
        isA<StoryCatalogNpcChoice>()
            .having(
              (choice) => choice.authoringQualification,
              'authoring qualification',
              AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
            )
            .having(
              (choice) => choice.runtimeQualification,
              'runtime qualification',
              AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
            )
            .having((choice) => choice.blocksBuild, 'blocks build', isTrue),
      ),
    );

    final asghan = adapter.createNpcDraftInput(
      catalogId: _asghanId,
      displayName: 'Asghan clone',
      moduleNamespace: 'GoreMods.Npcs.AsghanClone',
      uniqueName: 'GoreAsghanClone',
    );
    final character = _decode(asghan.parentCharacterDefinition);
    final ai = _decode(asghan.parentAiAgentConfig);
    final spawn = _decode(asghan.parentSpawnDefinition);
    _expectGeneration(character);
    expect(character.keys, <String>[
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_class',
    ]);
    expect(character['catalog_layer'], 'base-game.g1r.scripts');
    expect(
      character['canonical_selector'],
      _selectorAlias(_asghanId, 'character_definition'),
    );
    expect(
      character['runtime_class'],
      'UCharacterDefinition_Human_OM_GRD_Asghan_263',
    );
    expect(character['source_seal'], _seal('a', 100));
    expect(
      ai['canonical_selector'],
      _selectorAlias(_asghanId, 'ai_agent_config'),
    );
    expect(ai['source_seal'], _seal('b', 100));
    expect(
      spawn['canonical_selector'],
      _selectorAlias(_asghanId, 'spawn_definition'),
    );
    expect(spawn['source_seal'], _seal('c', 100));
    expect(
      asghan.parentCharacterDefinition.canonicalJson,
      isNot(contains('source_catalog_selector')),
    );
    expect(
      asghan.parentCharacterDefinition.canonicalJson,
      isNot(contains('Trusted/')),
    );

    final viper = adapter.createNpcDraftInput(
      catalogId: _viperId,
      displayName: 'Viper clone',
      moduleNamespace: 'GoreMods.Npcs.ViperClone',
      uniqueName: 'GoreViperClone',
    );
    final viperCharacter = _decode(viper.parentCharacterDefinition);
    expect(
      viperCharacter['runtime_class'],
      'UCharacterDefinition_Human_OM_STT_Viper_302',
    );
    expect(viperCharacter['source_seal'], _seal('e', 100));
  });

  test('catalog NPC input is accepted by the closed mutation factory', () {
    final input = adapter.createNpcDraftInput(
      catalogId: _asghanId,
      displayName: 'NPC GoreAsghanClone',
      moduleNamespace: 'GoreMods.Npcs.GoreAsghanClone',
      uniqueName: 'GoreAsghanClone',
    );
    final mutationJson = buildNpcStoryDraftMutationJson(
      context: StoryDraftMutationContext(
        projectId: '01010101010101010101010101010101',
        revision: 4,
        ids: StoryDraftEntityIds(
          draftId: '10101010101010101010101010101010',
          scriptModuleId: '11111111111111111111111111111111',
        ),
      ),
      input: input,
    );
    final mutation = (jsonDecode(mutationJson) as Map).cast<String, Object?>();
    final draft = (mutation['draft'] as Map).cast<String, Object?>();
    final encodedInput = (draft['input'] as Map).cast<String, Object?>();

    expect(draft['kind'], 'npc');
    expect(encodedInput['unique_name'], 'GoreAsghanClone');
    expect(
      encodedInput['parent_character_definition'],
      _decode(input.parentCharacterDefinition),
    );
    expect(
      encodedInput['parent_ai_agent_config'],
      _decode(input.parentAiAgentConfig),
    );
    expect(
      encodedInput['parent_spawn_definition'],
      _decode(input.parentSpawnDefinition),
    );
  });

  test('unknown NPC choice and oversized friendly input fail closed', () {
    expect(
      () => adapter.createNpcDraftInput(
        catalogId: 'g1r:npc:not-present',
        displayName: 'Unknown',
        moduleNamespace: 'GoreMods.Npcs.Unknown',
        uniqueName: 'GoreUnknown',
      ),
      throwsA(isA<StoryCatalogAdapterException>()),
    );
    expect(
      () => adapter.createNpcDraftInput(
        catalogId: _asghanId,
        displayName: 'Bounded',
        moduleNamespace: 'GoreMods.Npcs.Bounded',
        uniqueName: List<String>.filled(65, 'x').join(),
      ),
      throwsA(isA<StoryCatalogAdapterException>()),
    );
  });

  test('Quest choices are typed but creation stays disabled', () {
    final availability = adapter.questAvailability;

    expect(availability.canCreate, isFalse);
    expect(
      availability.disabledReason,
      StoryQuestDraftDisabledReason.collisionInventoryUnavailable,
    );
    expect(availability.parents, hasLength(1));
    expect(
      availability.parents.single.runtimeClass,
      'UQuest_SwampCamp_SCCHAPTER2',
    );
    expect(
      availability.parents.single.authoringSelector,
      _selectorAlias('g1r:quest-parent:swampcamp_scchapter2', 'quest_parent'),
    );
    expect(availability.givers, hasLength(2));
    expect(
      availability.givers.map((giver) => giver.runtimeUniqueName),
      <String>['OM_GRD_Asghan_263', 'OM_STT_Viper_302'],
    );
    expect(availability.collisionCatalogLayer, 'resolved-loadout.scripts.v1');
    expect(availability.collisionSourceSeal.byteLength, 123394250);
    expect(
      availability.collisionSourceSeal.sha256,
      List.filled(64, '2').join(),
    );
    expect(
      () => availability.parents.add(availability.parents.single),
      throwsUnsupportedError,
    );
    expect(
      () => adapter.npcChoices.add(adapter.npcChoices.first),
      throwsUnsupportedError,
    );
  });

  test(
    'inconsistent readiness is rejected before adapter construction',
    () async {
      final response = _catalogResponse();
      final selections = (response['selections'] as Map)
          .cast<String, Object?>();
      final collision = (selections['quest_collision_catalog'] as Map)
          .cast<String, Object?>();
      collision['blocks_draft_creation'] = false;

      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: <String, Map<String, Object?>>{
              'authoring_story_catalog_v1_read': response,
            },
          ),
        ).authoringStoryCatalogV1Read(catalogJson: _catalogJson),
        throwsFormatException,
      );
    },
  );
}

Future<AuthoringStoryCatalogSelections> _selections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': _catalogResponse(),
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogJson);

Map<String, Object?> _decode(CanonicalUnverifiedStoryJsonObject value) =>
    (jsonDecode(value.canonicalJson) as Map).cast<String, Object?>();

void _expectGeneration(Map<String, Object?> parent) {
  final generation = (parent['generation'] as Map).cast<String, Object?>();
  expect(generation.keys, <String>['executable']);
  expect(generation['executable'], _seal('1', 171698176));
}

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(64, byte).join(),
};

String _selectorAlias(String catalogId, String role) {
  final bytes = <int>[
    ...utf8.encode('gore-story-catalog.authoring-selector-v1\u0000'),
  ];
  for (final value in <String>[catalogId, role]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

Map<String, Object?> _classSelection(
  String catalogId,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _selectorAlias(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _seal(sealByte, 100),
};

Map<String, Object?> _npc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper ? _viperId : _asghanId;
  final character = _classSelection(
    catalogId,
    'character_definition',
    viper ? 'e' : 'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _classSelection(
      catalogId,
      'ai_agent_config',
      viper ? 'd' : 'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _classSelection(
      catalogId,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _selectorAlias(catalogId, 'quest_giver'),
      'source_catalog_selector': character['source_catalog_selector'],
      'runtime_unique_name': runtime,
      'source_seal': character['source_seal'],
    },
    'discovery_status': 'sealed_cache_defaults_verified',
    'authoring_qualification': 'offline_qualified',
    'runtime_qualification': 'runtime_unqualified',
    'evidence_id': viper
        ? 'npc-logical-clone-v1:viper-current-v1'
        : 'npc-logical-clone-v1',
    'blocks_build': true,
  };
}

Map<String, Object?> _catalogResponse() => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(_catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': <String, Object?>{
      'edition': 'g1r-steam',
      'executable': _seal('1', 171698176),
      'shipping_cache': _seal('2', 123394250),
      'binds_cache': _seal('3', 5903938),
    },
    'catalog_seal': _seal('4', 5611),
    'npcs': <Object?>[_npc(viper: false), _npc(viper: true)],
    'quest_parents': <Object?>[
      <String, Object?>{
        'catalog_id': 'g1r:quest-parent:swampcamp_scchapter2',
        'display_name': 'Swamp Camp Chapter 2',
        'quest_class': _classSelection(
          'g1r:quest-parent:swampcamp_scchapter2',
          'quest_parent',
          'f',
          'UQuest_SwampCamp_SCCHAPTER2',
        ),
        'parent_class_name': 'UQuest_SwampCamp',
        'role': 'chapter',
        'qualification': 'curated_defaults_verified',
        'transition_qualification': 'runtime_unqualified',
        'evidence_id': 'current-cache-defaults-swampcamp-chapter2-20260712',
        'blocks_build': true,
      },
    ],
    'quest_collision_catalog': <String, Object?>{
      'status': 'inventory_unavailable',
      'catalog_layer': 'resolved-loadout.scripts.v1',
      'source_seal': _seal('2', 123394250),
      'blocks_draft_creation': true,
    },
    'blocks_build': true,
  },
};
