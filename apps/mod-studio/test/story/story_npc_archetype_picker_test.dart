import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_npc_archetype_index.dart';
import 'package:gore_mod/story/ui/story_npc_archetype_picker.dart';

const _catalogRequest = '{"format":"story_catalog"}';
const _gameRoot = r'C:\Games\Gothic Remake';
const _npcCommand = 'authoring_npc_archetype_catalog_v1_build_for_game_root';
const _npcBindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';
const _selectorDomain = 'gore-story-catalog.authoring-selector-v1\u0000';
const _asghanId = 'g1r:npc:asghan';
const _viperId = 'g1r:npc:viper';
const _asghanSpawn = 'USpawnAIAgentDefinition_000_Asghan';
const _asghanAi = 'UAIAgentConfig_000_Asghan';
const _asghanCharacter = 'UCharacterDefinition_Human_ASGHAN';
const _viperSpawn = 'USpawnAIAgentDefinition_999_Viper';
const _viperAi = 'UAIAgentConfig_999_Viper';
const _viperCharacter = 'UCharacterDefinition_Human_VIPER';

const _labels = StoryNpcArchetypePickerLabels(
  title: 'Choose NPC archetype',
  search: 'Search archetypes',
  showExperimental: 'Show experimental archetypes',
  offlineQualified: 'Offline qualified',
  experimentalStaticLinkage: 'Experimental · static linkage only',
  empty: 'No archetypes match.',
  spawnClass: 'Spawn class',
  aiConfigClass: 'AI config',
  characterDefinitionClass: 'Character definition',
  actorBlueprint: 'Actor Blueprint',
  bodyBlueprintFamily: 'Body/Blueprint family',
  humanBaseFamily: 'Human base',
  humanWomanFamily: 'Human woman',
  otherFamily: 'Other',
);

void main() {
  late StoryNpcArchetypeIndex index;

  setUpAll(() async {
    index = await _index(<Map<String, Object?>>[
      _record(
        spawn: _asghanSpawn,
        ai: _asghanAi,
        character: _asghanCharacter,
        actor: 'BP_Asghan',
        spawnSeal: _fixedSeal('c', 10),
        aiSeal: _fixedSeal('b', 10),
        characterSeal: _fixedSeal('a', 10),
      ),
      _record(
        spawn: 'USpawnAIAgentDefinition_500_Generic',
        ai: 'UAIAgentConfig_SearchableGuard',
        character: 'UCharacterDefinition_Generic',
        actor: 'BP_MineGuard',
        family: 'human_woman',
        spawnSeal: _fixedSeal('1', 10),
        aiSeal: _fixedSeal('2', 10),
        characterSeal: _fixedSeal('3', 10),
      ),
      _record(
        spawn: _viperSpawn,
        ai: _viperAi,
        character: _viperCharacter,
        actor: 'BP_Viper',
        spawnSeal: _fixedSeal('f', 10),
        aiSeal: _fixedSeal('e', 10),
        characterSeal: _fixedSeal('d', 10),
      ),
    ]);
  });

  testWidgets('qualified rows select only a curated catalog ID', (
    tester,
  ) async {
    String? selected;
    await _pump(tester, index: index, onSelected: (value) => selected = value);

    expect(find.text('Asghan'), findsOneWidget);
    expect(find.text('Viper'), findsOneWidget);
    expect(find.text('USpawnAIAgentDefinition_500_Generic'), findsNothing);
    expect(find.text(_labels.offlineQualified), findsNWidgets(2));

    await tester.tap(find.text('Asghan'));
    expect(selected, _asghanId);
  });

  testWidgets('experimental rows are opt-in, searchable, and disabled', (
    tester,
  ) async {
    String? selected;
    await _pump(tester, index: index, onSelected: (value) => selected = value);

    await tester.tap(
      find.byKey(const Key('story-npc-archetype-show-experimental')),
    );
    await tester.pump();
    expect(find.text('USpawnAIAgentDefinition_500_Generic'), findsOneWidget);
    expect(find.text(_labels.experimentalStaticLinkage), findsOneWidget);
    expect(find.textContaining('Human woman'), findsOneWidget);

    await tester.tap(find.text('USpawnAIAgentDefinition_500_Generic'));
    expect(selected, isNull);

    await tester.enterText(
      find.byKey(const Key('story-npc-archetype-search')),
      'searchableguard mineguard',
    );
    await tester.pump();
    expect(find.text('USpawnAIAgentDefinition_500_Generic'), findsOneWidget);
    expect(find.text('Asghan'), findsNothing);
  });

  testWidgets('empty state is explicit and clearing search restores rows', (
    tester,
  ) async {
    await _pump(tester, index: index, onSelected: (_) {});
    final search = find.byKey(const Key('story-npc-archetype-search'));
    await tester.enterText(search, 'does-not-exist');
    await tester.pump();
    expect(find.byKey(const Key('story-npc-archetype-empty')), findsOneWidget);
    expect(find.text(_labels.empty), findsOneWidget);

    await tester.enterText(search, 'viper');
    await tester.pump();
    expect(find.text('Viper'), findsOneWidget);
  });

  testWidgets('dialog returns the curated ID', (tester) async {
    String? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => FilledButton(
            onPressed: () async {
              result = await showStoryNpcArchetypePicker(
                context: context,
                index: index,
                labels: _labels,
              );
            },
            child: const Text('Open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text(_labels.title), findsOneWidget);
    await tester.tap(find.text('Viper'));
    await tester.pumpAndSettle();
    expect(result, _viperId);
  });

  testWidgets('results use a lazy viewport', (tester) async {
    final records = <Map<String, Object?>>[
      _record(
        spawn: _asghanSpawn,
        ai: _asghanAi,
        character: _asghanCharacter,
        actor: 'BP_Asghan',
        spawnSeal: _fixedSeal('c', 10),
        aiSeal: _fixedSeal('b', 10),
        characterSeal: _fixedSeal('a', 10),
      ),
      for (var value = 1; value < 199; value++)
        _record(
          spawn:
              'USpawnAIAgentDefinition_${value.toString().padLeft(3, '0')}_Generic',
          ai: 'UAIAgentConfig_${value.toString().padLeft(3, '0')}',
          character: 'UCharacterDefinition_${value.toString().padLeft(3, '0')}',
          actor: 'BP_${value.toString().padLeft(3, '0')}',
          spawnSeal: _fixedSeal('1', 10),
          aiSeal: _fixedSeal('2', 10),
          characterSeal: _fixedSeal('3', 10),
        ),
      _record(
        spawn: _viperSpawn,
        ai: _viperAi,
        character: _viperCharacter,
        actor: 'BP_Viper',
        spawnSeal: _fixedSeal('f', 10),
        aiSeal: _fixedSeal('e', 10),
        characterSeal: _fixedSeal('d', 10),
      ),
    ];
    final large = await _index(records);
    await _pump(tester, index: large, onSelected: (_) {});
    await tester.tap(
      find.byKey(const Key('story-npc-archetype-show-experimental')),
    );
    await tester.pump();

    expect(find.byType(ListTile), findsWidgets);
    expect(find.byType(ListTile).evaluate().length, lessThan(records.length));
    expect(
      find.byKey(
        const ValueKey<String>(
          'story-npc-archetype-USpawnAIAgentDefinition_198_Generic',
        ),
      ),
      findsNothing,
    );
  });
}

Future<void> _pump(
  WidgetTester tester, {
  required StoryNpcArchetypeIndex index,
  required ValueChanged<String> onSelected,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SizedBox(
        width: 900,
        height: 620,
        child: StoryNpcArchetypePicker(
          index: index,
          labels: _labels,
          onSelected: onSelected,
        ),
      ),
    ),
  ),
);

Future<StoryNpcArchetypeIndex> _index(
  List<Map<String, Object?>> records,
) async => StoryNpcArchetypeIndex.fromCatalogs(
  story: await _storySelections(),
  archetypes: await _archetypeCatalog(records),
);

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
              catalogId: _asghanId,
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
              catalogId: _viperId,
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
  const id = 'g1r:quest-parent:chapter';
  return <String, Object?>{
    'catalog_id': id,
    'display_name': 'Chapter',
    'quest_class': _storyClass(
      id,
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

Future<AuthoringNpcArchetypeCatalogBuildResult> _archetypeCatalog(
  List<Map<String, Object?>> records,
) {
  final generation = _generation();
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
    'story_catalog_seal': _storyCatalogSeal(),
    'qualification': _qualification(),
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
    'request_binding_sha256': _requestBinding(_gameRoot),
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

Map<String, Object?> _record({
  required String spawn,
  required String ai,
  required String character,
  required String actor,
  required Map<String, Object?> spawnSeal,
  required Map<String, Object?> aiSeal,
  required Map<String, Object?> characterSeal,
  String family = 'human_base',
}) => <String, Object?>{
  'spawn': _npcClass(spawn, 'USpawn', spawnSeal),
  'ai_config': _npcClass(ai, 'UAi', aiSeal),
  'character_definition': _npcClass(character, 'UCharacter', characterSeal),
  'actor_blueprint': actor,
  'blueprint_family': family,
  'spawn_ai_edge': _edge(spawn, 'AIAgentConfigClass', ai, '1'),
  'spawn_blueprint_edge': _edge(spawn, 'AIAgentCharacterClass', actor, '2'),
  'ai_character_edge': _edge(ai, 'm_CharacterDefinition', character, '3'),
  'evidence_sha256': _hex('7'),
};

Map<String, Object?> _npcClass(
  String name,
  String parent,
  Map<String, Object?> seal,
) => <String, Object?>{
  'class_name': name,
  'super_class': parent,
  'module_name': 'World',
  'relative_path': 'World/$name.as',
  'source_seal': _deepCopy(seal),
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

Map<String, Object?> _qualification() => <String, Object?>{
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

String _requestBinding(String root) {
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
