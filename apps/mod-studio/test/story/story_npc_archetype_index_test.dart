import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_npc_archetype_index.dart';

const _catalogRequest = '{"format":"story_catalog"}';
const _gameRoot = r'C:\Games\Gothic Remake';
const _npcCommand = 'authoring_npc_archetype_catalog_v1_build_for_game_root';
const _npcBindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';
const _selectorDomain = 'gore-story-catalog.authoring-selector-v1\u0000';
const _storyCatalogIdAsghan = 'g1r:npc:asghan';
const _storyCatalogIdViper = 'g1r:npc:viper';
const _asghanSpawn = 'USpawnAIAgentDefinition_A_Asghan';
const _asghanAi = 'UAIAgentConfig_A_Asghan';
const _asghanCharacter = 'UCharacterDefinition_Human_ASGHAN';
const _genericSpawn = 'USpawnAIAgentDefinition_B_Generic';
const _genericAi = 'UAIAgentConfig_B_Generic';
const _genericCharacter = 'UCharacterDefinition_B_Generic';
const _viperSpawn = 'USpawnAIAgentDefinition_C_Viper';
const _viperAi = 'UAIAgentConfig_C_Viper';
const _viperCharacter = 'UCharacterDefinition_Human_VIPER';

void main() {
  late AuthoringStoryCatalogSelections story;
  late AuthoringNpcArchetypeCatalogBuildResult archetypes;

  setUp(() async {
    story = await _storySelections();
    archetypes = await _archetypeCatalog(
      records: _defaultRecords(),
      generation: _generation(),
      storyCatalogSeal: _storyCatalogSeal(),
    );
  });

  test('join fails closed on generation or Story catalog mismatch', () async {
    final otherGeneration = _generation();
    (otherGeneration['executable'] as Map<String, Object?>)['sha256'] = _hex(
      '9',
    );
    final wrongGeneration = await _archetypeCatalog(
      records: _defaultRecords(),
      generation: otherGeneration,
      storyCatalogSeal: _storyCatalogSeal(),
    );
    expect(
      () => StoryNpcArchetypeIndex.fromCatalogs(
        story: story,
        archetypes: wrongGeneration,
      ),
      throwsA(isA<StoryNpcArchetypeIndexException>()),
    );

    final wrongStorySeal = await _archetypeCatalog(
      records: _defaultRecords(),
      generation: _generation(),
      storyCatalogSeal: _fixedSeal('8', 5611),
    );
    expect(
      () => StoryNpcArchetypeIndex.fromCatalogs(
        story: story,
        archetypes: wrongStorySeal,
      ),
      throwsA(isA<StoryNpcArchetypeIndexException>()),
    );
  });

  test('only exact Asghan and Viper evidence is promoted', () {
    final index = StoryNpcArchetypeIndex.fromCatalogs(
      story: story,
      archetypes: archetypes,
    );

    expect(index.rows.map((row) => row.spawnClass), <String>[
      _asghanSpawn,
      _genericSpawn,
      _viperSpawn,
    ]);
    final asghan = index.rows[0];
    final generic = index.rows[1];
    final viper = index.rows[2];
    expect(asghan.label, 'Asghan');
    expect(asghan.selectable, isTrue);
    expect(
      asghan.qualification,
      StoryNpcArchetypeQualification.offlineCloneQualified,
    );
    expect(asghan.curatedCatalogId, _storyCatalogIdAsghan);
    expect(viper.label, 'Viper');
    expect(viper.selectable, isTrue);
    expect(viper.curatedCatalogId, _storyCatalogIdViper);
    expect(generic.label, _genericSpawn);
    expect(generic.curatedDisplayName, isNull);
    expect(generic.curatedRuntimeUniqueName, isNull);
    expect(generic.experimental, isTrue);
    expect(generic.selectable, isFalse);
    expect(generic.curatedCatalogId, isNull);
    expect(
      generic.bodyBlueprintFamilyLabel,
      'Human woman body/blueprint family',
    );
    expect(index.selectableForCatalogId(_storyCatalogIdAsghan), same(asghan));
  });

  test('one class-source near match is never promoted', () async {
    final near = _defaultRecords();
    final asghanCharacter =
        near.first['character_definition'] as Map<String, Object?>;
    asghanCharacter['source_seal'] = _fixedSeal('7', 10);
    final nearCatalog = await _archetypeCatalog(
      records: near,
      generation: _generation(),
      storyCatalogSeal: _storyCatalogSeal(),
    );
    final index = StoryNpcArchetypeIndex.fromCatalogs(
      story: story,
      archetypes: nearCatalog,
    );

    final asghan = index.rows.first;
    expect(asghan.spawnClass, _asghanSpawn);
    expect(asghan.experimental, isTrue);
    expect(asghan.selectable, isFalse);
    expect(asghan.curatedCatalogId, isNull);
    expect(index.selectableForCatalogId(_storyCatalogIdAsghan), isNull);
    expect(index.selectableForCatalogId(_storyCatalogIdViper), isNotNull);
  });

  test(
    'search hides experimental rows by default and is Unicode/case aware',
    () {
      final index = StoryNpcArchetypeIndex.fromCatalogs(
        story: story,
        archetypes: archetypes,
      );

      expect(index.search('').map((row) => row.label), <String>[
        'Asghan',
        'Viper',
      ]);
      expect(index.search('generic'), isEmpty);
      expect(
        index.search('generic', includeExperimental: true).single.spawnClass,
        _genericSpawn,
      );
      expect(
        index.search('ÄTHER', includeExperimental: true).single.actorBlueprint,
        'Blueprint_Äther_𐐀',
      );
      expect(
        index.search('vIpEr').single.curatedCatalogId,
        _storyCatalogIdViper,
      );
      expect(
        index.search('asghan uai').single.curatedCatalogId,
        _storyCatalogIdAsghan,
      );
    },
  );

  test('rows and every search projection are immutable and preserve order', () {
    final index = StoryNpcArchetypeIndex.fromCatalogs(
      story: story,
      archetypes: archetypes,
    );
    expect(() => index.rows.add(index.rows.first), throwsUnsupportedError);
    final all = index.search('', includeExperimental: true);
    expect(all.map((row) => row.spawnClass), <String>[
      _asghanSpawn,
      _genericSpawn,
      _viperSpawn,
    ]);
    expect(() => all.clear(), throwsUnsupportedError);
  });
}

Future<AuthoringStoryCatalogSelections> _storySelections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': <String, Object?>{
        'ok': true,
        'request_catalog_sha256': crypto.sha256
            .convert(utf8.encode(_catalogRequest))
            .toString(),
        'selections': <String, Object?>{
          'schema_revision': 1,
          'generation': _generation(),
          'catalog_seal': _storyCatalogSeal(),
          'npcs': <Object?>[
            _storyNpc(
              catalogId: _storyCatalogIdAsghan,
              displayName: 'Asghan',
              runtimeUniqueName: 'ASGHAN',
              spawn: _asghanSpawn,
              ai: _asghanAi,
              character: _asghanCharacter,
              spawnSeal: _fixedSeal('c', 10),
              aiSeal: _fixedSeal('b', 10),
              characterSeal: _fixedSeal('a', 10),
            ),
            _storyNpc(
              catalogId: _storyCatalogIdViper,
              displayName: 'Viper',
              runtimeUniqueName: 'VIPER',
              spawn: _viperSpawn,
              ai: _viperAi,
              character: _viperCharacter,
              spawnSeal: _fixedSeal('f', 10),
              aiSeal: _fixedSeal('e', 10),
              characterSeal: _fixedSeal('d', 10),
            ),
          ],
          'quest_parents': <Object?>[_questParent()],
          'quest_collision_catalog': <String, Object?>{
            'status': 'inventory_unavailable',
            'catalog_layer': 'resolved-loadout.scripts.v1',
            'source_seal': _deepCopy(_generation()['shipping_cache']!),
            'blocks_draft_creation': true,
          },
          'blocks_build': true,
        },
      },
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogRequest);

Map<String, Object?> _storyNpc({
  required String catalogId,
  required String displayName,
  required String runtimeUniqueName,
  required String spawn,
  required String ai,
  required String character,
  required Map<String, Object?> spawnSeal,
  required Map<String, Object?> aiSeal,
  required Map<String, Object?> characterSeal,
}) => <String, Object?>{
  'catalog_id': catalogId,
  'display_name': displayName,
  'runtime_unique_name': runtimeUniqueName,
  'character_definition': _storyClass(
    catalogId,
    'character_definition',
    character,
    characterSeal,
  ),
  'ai_agent_config': _storyClass(catalogId, 'ai_agent_config', ai, aiSeal),
  'spawn_definition': _storyClass(
    catalogId,
    'spawn_definition',
    spawn,
    spawnSeal,
  ),
  'quest_giver': <String, Object?>{
    'catalog_layer': 'base-game.g1r.scripts',
    'authoring_selector': _selector(catalogId, 'quest_giver'),
    'source_catalog_selector': 'script-class:Trusted/$character',
    'runtime_unique_name': runtimeUniqueName,
    'source_seal': _deepCopy(characterSeal),
  },
  'discovery_status': 'sealed_cache_defaults_verified',
  'authoring_qualification': 'offline_qualified',
  'runtime_qualification': 'runtime_unqualified',
  'evidence_id': 'g1r:evidence:${runtimeUniqueName.toLowerCase()}',
  'blocks_build': true,
};

Map<String, Object?> _storyClass(
  String catalogId,
  String role,
  String runtimeClass,
  Map<String, Object?> sourceSeal,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _selector(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _deepCopy(sourceSeal),
};

Map<String, Object?> _questParent() {
  const catalogId = 'g1r:quest-parent:chapter';
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': 'Chapter',
    'quest_class': _storyClass(
      catalogId,
      'quest_parent',
      'UQuest_Chapter',
      _fixedSeal('6', 10),
    ),
    'parent_class_name': 'UQuest_Base',
    'role': 'chapter',
    'qualification': 'curated_defaults_verified',
    'transition_qualification': 'runtime_unqualified',
    'evidence_id': 'g1r:evidence:chapter',
    'blocks_build': true,
  };
}

Future<AuthoringNpcArchetypeCatalogBuildResult> _archetypeCatalog({
  required List<Map<String, Object?>> records,
  required Map<String, Object?> generation,
  required Map<String, Object?> storyCatalogSeal,
}) {
  final sourceIdentity = <String, Object?>{
    'shipping_cache': _deepCopy(generation['shipping_cache']!),
    'binds_cache': _deepCopy(generation['binds_cache']!),
  };
  final source = <String, Object?>{
    ...sourceIdentity,
    'source_pair_seal': _sealJson(jsonEncode(sourceIdentity)),
  };
  final payload = <String, Object?>{
    'extractor_records_sha256': _hex('4'),
    'records': records,
    'rejections': <Object?>[],
  };
  final catalog = <String, Object?>{
    'generation': generation,
    'story_catalog_seal': storyCatalogSeal,
    'qualification': _npcQualification(),
    'source': source,
    'payload': payload,
    'payload_seal': _sealJson(jsonEncode(payload)),
  };
  final artifact = <String, Object?>{
    'format': 'npc_archetype_catalog',
    'schema_revision': 1,
    'catalog': catalog,
    'catalog_seal': _sealJson(jsonEncode(catalog)),
  };
  final response = <String, Object?>{
    'ok': true,
    'request_binding_sha256': _npcRequestBinding(_gameRoot),
    'catalog_json': jsonEncode(artifact),
    'generation': _deepCopy(generation),
    'catalog_seal': _deepCopy(artifact['catalog_seal']!),
    'source': _deepCopy(source),
    'payload_seal': _deepCopy(catalog['payload_seal']!),
    'record_count': records.length,
    'rejection_count': 0,
    'qualification': _deepCopy(catalog['qualification']!),
  };
  return ModFfi(
    FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_npcCommand: response},
    ),
  ).authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: _gameRoot);
}

List<Map<String, Object?>> _defaultRecords() => <Map<String, Object?>>[
  _npcRecord(
    spawn: _asghanSpawn,
    ai: _asghanAi,
    character: _asghanCharacter,
    actorBlueprint: 'Blueprint_Asghan',
    family: 'human_base',
    spawnSeal: _fixedSeal('c', 10),
    aiSeal: _fixedSeal('b', 10),
    characterSeal: _fixedSeal('a', 10),
  ),
  _npcRecord(
    spawn: _genericSpawn,
    ai: _genericAi,
    character: _genericCharacter,
    actorBlueprint: 'Blueprint_Äther_𐐀',
    family: 'human_woman',
    spawnSeal: _fixedSeal('1', 10),
    aiSeal: _fixedSeal('2', 10),
    characterSeal: _fixedSeal('3', 10),
  ),
  _npcRecord(
    spawn: _viperSpawn,
    ai: _viperAi,
    character: _viperCharacter,
    actorBlueprint: 'Blueprint_Viper',
    family: 'human_base',
    spawnSeal: _fixedSeal('f', 10),
    aiSeal: _fixedSeal('e', 10),
    characterSeal: _fixedSeal('d', 10),
  ),
];

Map<String, Object?> _npcRecord({
  required String spawn,
  required String ai,
  required String character,
  required String actorBlueprint,
  required String family,
  required Map<String, Object?> spawnSeal,
  required Map<String, Object?> aiSeal,
  required Map<String, Object?> characterSeal,
}) => <String, Object?>{
  'spawn': _npcClass(spawn, 'USpawnAIAgentDefinition', spawnSeal),
  'ai_config': _npcClass(ai, 'UAIAgentConfig', aiSeal),
  'character_definition': _npcClass(
    character,
    'UCharacterDefinition',
    characterSeal,
  ),
  'actor_blueprint': actorBlueprint,
  'blueprint_family': family,
  'spawn_ai_edge': _edge(spawn, 'AIAgentConfigClass', ai, '1'),
  'spawn_blueprint_edge': _edge(
    spawn,
    'AIAgentCharacterClass',
    actorBlueprint,
    '2',
  ),
  'ai_character_edge': _edge(ai, 'm_CharacterDefinition', character, '3'),
  'evidence_sha256': _hex('7'),
};

Map<String, Object?> _npcClass(
  String className,
  String superClass,
  Map<String, Object?> sourceSeal,
) => <String, Object?>{
  'class_name': className,
  'super_class': superClass,
  'module_name': 'World',
  'relative_path': 'World/$className.as',
  'source_seal': _deepCopy(sourceSeal),
};

Map<String, Object?> _edge(
  String owner,
  String field,
  String assigned,
  String digest,
) => <String, Object?>{
  'owner_class': owner,
  'field_name': field,
  'assigned_value': assigned,
  'instruction_offset_dwords': 1,
  'init_defaults_bytecode_seal': _fixedSeal(digest, 20),
  'evidence_sha256': _hex(digest),
};

Map<String, Object?> _generation() => <String, Object?>{
  'edition': 'g1r-steam',
  'executable': _fixedSeal('a', 100),
  'shipping_cache': _fixedSeal('b', 200),
  'binds_cache': _fixedSeal('c', 300),
};

Map<String, Object?> _storyCatalogSeal() => _fixedSeal('d', 5611);

Map<String, Object?> _npcQualification() => <String, Object?>{
  'linkage': 'sealed_linkage_verified',
  'runtime': 'runtime_unqualified',
  'build': 'not_supported',
  'deploy': 'not_supported',
  'publication': 'not_supported',
};

Map<String, Object?> _fixedSeal(String digit, int byteLength) =>
    <String, Object?>{'byte_len': byteLength, 'sha256': _hex(digit)};

Map<String, Object?> _sealJson(String value) {
  final bytes = utf8.encode(value);
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

String _selector(String catalogId, String role) {
  final bytes = <int>[...utf8.encode(_selectorDomain)];
  for (final value in <String>[catalogId, role]) {
    final part = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, part.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(part);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

String _npcRequestBinding(String root) {
  final bytes = utf8.encode(root);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(_npcBindingDomain),
    ...length,
    ...bytes,
  ]).toString();
}

String _hex(String digit) => List<String>.filled(64, digit).join();

Object _deepCopy(Object value) => jsonDecode(jsonEncode(value))!;
