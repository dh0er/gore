import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_base_game_content_browser.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/story/domain/story_catalog_adapter.dart';
import 'package:gore_mod/story/domain/story_npc_archetype_index.dart';

const _catalogRequest = '{"format":"story_catalog"}';
const _gameRoot = r'C:\Games\Gothic Remake';
const _storyBuildCommand = 'authoring_story_catalog_v1_build_for_game_root';
const _storyReadCommand = 'authoring_story_catalog_v1_read';
const _npcCommand = 'authoring_npc_archetype_catalog_v1_build_for_game_root';
const _storyBuildBindingDomain =
    'gore-story-catalog.authoring-build-for-game-root-v1.request-binding\u0000';
const _npcBindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';
const _selectorDomain = 'gore-story-catalog.authoring-selector-v1\u0000';
const _asghanId = 'g1r:npc:asghan';
const _viperId = 'g1r:npc:viper';
const _questId = 'g1r:quest-parent:chapter';
const _asghanSpawn = 'USpawnAIAgentDefinition_000_Asghan';
const _asghanAi = 'UAIAgentConfig_000_Asghan';
const _asghanCharacter = 'UCharacterDefinition_Human_ASGHAN';
const _viperSpawn = 'USpawnAIAgentDefinition_999_Viper';
const _viperAi = 'UAIAgentConfig_999_Viper';
const _viperCharacter = 'UCharacterDefinition_Human_VIPER';
const _genericSpawn = 'USpawnAIAgentDefinition_500_Generic';

const _copy = Revision3BaseGameContentBrowserCopy(
  title: 'Base game content',
  description: 'Fresh, read-only game catalog.',
  missingGameTitle: 'Game installation required',
  missingGameDescription: 'Configure the game before browsing.',
  configureGame: 'Configure game',
  loading: 'Reading base game catalog',
  refresh: 'Refresh',
  searchLabel: 'Search base game content',
  filterAll: 'All',
  filterNpcs: 'NPCs',
  filterQuests: 'Quests',
  npcSectionTitle: 'Curated NPC archetypes',
  questSectionTitle: 'Quest families',
  experimentalNpcSectionTitle: 'Experimental NPC evidence',
  searchForExperimental: 'Search to inspect the broad NPC inventory.',
  empty: 'No base game content matches.',
  loadErrorTitle: 'Base game catalog unavailable',
  loadErrorDescription: 'No empty catalog was substituted.',
  retry: 'Retry',
  baseGameSourceBadge: 'Base game',
  offlineDraftBadge: 'Offline Draft',
  runtimeUnqualifiedBadge: 'Runtime unqualified',
  inspectOnlyBadge: 'Inspect only',
  createNpcDraft: 'Create NPC Draft',
  createQuestDraft: 'Create Quest Draft',
  spawnClass: 'Spawn class',
  actorBlueprint: 'Actor Blueprint',
  experimentalResultsCapped: 'Refine the search to see more results.',
);

void main() {
  late Revision3BaseGameContentCatalog catalog;

  setUpAll(() async {
    catalog = await _catalog();
  });

  test(
    'service scans one exact evidence snapshot for both child catalogs',
    () async {
      final core = _baseGameCatalogCore();

      final result = await Revision3BaseGameContentCatalogService(
        ModFfi(core),
      ).load(_gameRoot);

      expect(core.calls.map((call) => call.command), <String>[
        _storyBuildCommand,
        _npcCommand,
        _storyReadCommand,
      ]);
      for (final command in <String>[
        _storyBuildCommand,
        _storyReadCommand,
        _npcCommand,
      ]) {
        expect(
          core.calls.where((call) => call.command == command),
          hasLength(1),
          reason: '$command must read the snapshot exactly once',
        );
      }
      expect(
        core.calls
            .where(
              (call) =>
                  call.command == _storyBuildCommand ||
                  call.command == _npcCommand,
            )
            .map((call) => call.payload),
        everyElement(<String, Object?>{'game_root': _gameRoot}),
      );

      expect(result.npcs.choices.map((choice) => choice.catalogId), <String>[
        _asghanId,
        _viperId,
      ]);
      expect(
        result.quests.givers.map((choice) => choice.catalogId),
        result.npcs.choices.map((choice) => choice.catalogId),
      );
      expect(result.quests.parents.map((choice) => choice.catalogId), <String>[
        _questId,
      ]);
      final archetypeIndex = result.npcs.archetypeIndex!;
      expect(archetypeIndex.rows, hasLength(3));
      expect(archetypeIndex.selectableForCatalogId(_asghanId), isNotNull);
      expect(archetypeIndex.selectableForCatalogId(_viperId), isNotNull);
      expect(
        archetypeIndex.searchExperimental('searchableguard mineguard'),
        hasLength(1),
      );
      expect(result.quests.catalogSeal?.sha256, _storyCatalogSeal()['sha256']);
      expect(
        result.quests.generationExecutableSeal?.sha256,
        (_generation()['executable']! as Map<String, Object?>)['sha256'],
      );
    },
  );

  test('service rejects internally sealed cross-generation evidence', () async {
    final mismatchedGeneration = _generation();
    mismatchedGeneration['executable'] = _fixedSeal('9', 100);
    final core = _baseGameCatalogCore(
      archetypeGeneration: mismatchedGeneration,
    );

    await expectLater(
      Revision3BaseGameContentCatalogService(ModFfi(core)).load(_gameRoot),
      throwsA(
        isA<StoryNpcArchetypeIndexException>().having(
          (error) => error.message,
          'message',
          contains('different generations'),
        ),
      ),
    );
    for (final command in <String>[
      _storyBuildCommand,
      _storyReadCommand,
      _npcCommand,
    ]) {
      expect(
        core.calls.where((call) => call.command == command),
        hasLength(1),
        reason: '$command must complete before the sealed join rejects it',
      );
    }
  });

  testWidgets('missing game root never calls loader and opens settings', (
    tester,
  ) async {
    var loads = 0;
    var openedSettings = false;
    await _pump(
      tester,
      gameRoot: '   ',
      loader: (_) async {
        loads++;
        return catalog;
      },
      openSettings: () => openedSettings = true,
    );

    expect(loads, 0);
    expect(
      find.byKey(const Key('revision3-base-game-content-browser-missing-game')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(
        const Key('revision3-base-game-content-browser-open-settings'),
      ),
    );
    expect(openedSettings, isTrue);
  });

  testWidgets(
    'loads curated choices, searches inspect-only rows, and emits safe IDs',
    (tester) async {
      var loads = 0;
      String? npcId;
      String? questId;
      await _pump(
        tester,
        loader: (_) async {
          loads++;
          return catalog;
        },
        createNpcDraft: (value) => npcId = value,
        createQuestDraft: (value) => questId = value,
      );
      await tester.pumpAndSettle();

      expect(loads, 1);
      expect(find.text('Asghan'), findsOneWidget);
      expect(find.text('Viper'), findsOneWidget);
      expect(find.text(_genericSpawn), findsNothing);
      expect(find.text('Base game'), findsWidgets);
      expect(find.text('Offline Draft'), findsWidgets);
      expect(find.text('Runtime unqualified'), findsWidgets);

      await tester.tap(
        find.byKey(
          const ValueKey(('revision3-base-game-create-npc', _asghanId)),
        ),
      );
      expect(npcId, _asghanId);

      final questAction = find.byKey(
        const ValueKey(('revision3-base-game-create-quest', _questId)),
      );
      await tester.ensureVisible(questAction);
      await tester.pumpAndSettle();
      await tester.tap(questAction);
      expect(questId, _questId);

      await tester.fling(
        find.byType(Scrollable).first,
        const Offset(0, 1000),
        1000,
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-base-game-content-browser-search')),
        'searchableguard mineguard',
      );
      await tester.pump();
      final experimental = find.byKey(
        const ValueKey(('revision3-base-game-experimental-npc', _genericSpawn)),
      );
      await tester.ensureVisible(experimental);
      await tester.pumpAndSettle();

      expect(find.text(_genericSpawn), findsWidgets);
      expect(find.text('Inspect only'), findsOneWidget);
      expect(find.text('Create NPC Draft'), findsNothing);
      expect(find.text('Create Quest Draft'), findsNothing);
      expect(npcId, _asghanId);
      expect(questId, _questId);
    },
  );

  testWidgets('All, NPC, and Quest filters constrain curated results', (
    tester,
  ) async {
    await _pump(tester, loader: (_) async => catalog);
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-base-game-content-browser-filter-quest')),
    );
    await tester.pump();
    expect(find.text('Asghan'), findsNothing);
    expect(find.text('Chapter'), findsOneWidget);

    await tester.tap(
      find.byKey(const Key('revision3-base-game-content-browser-filter-npc')),
    );
    await tester.pump();
    expect(find.text('Asghan'), findsOneWidget);
    expect(find.text('Chapter'), findsNothing);
  });

  testWidgets('source changes reload and suppress stale completion', (
    tester,
  ) async {
    final first = Completer<Revision3BaseGameContentCatalog>();
    final second = Completer<Revision3BaseGameContentCatalog>();
    final loaders = <Completer<Revision3BaseGameContentCatalog>>[first, second];
    var calls = 0;
    final key = GlobalKey<_MutableHarnessState>();
    await tester.pumpWidget(
      _MutableHarness(key: key, loader: (_) => loaders[calls++].future),
    );
    expect(calls, 1);

    key.currentState!.changeSource('generation-b');
    await tester.pump();
    expect(calls, 2);

    first.complete(_simpleCatalog('STALE NPC'));
    await tester.pump();
    expect(find.text('STALE NPC'), findsNothing);
    expect(
      find.byKey(const Key('revision3-base-game-content-browser-loading')),
      findsOneWidget,
    );

    second.complete(_simpleCatalog('CURRENT NPC'));
    await tester.pumpAndSettle();
    expect(find.text('CURRENT NPC'), findsOneWidget);
    expect(find.text('STALE NPC'), findsNothing);
  });

  testWidgets('load error is explicit and retry can recover', (tester) async {
    var calls = 0;
    await _pump(
      tester,
      loader: (_) async {
        calls++;
        if (calls == 1) throw StateError('scan failed');
        return catalog;
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-base-game-content-browser-error')),
      findsOneWidget,
    );
    expect(find.text('No base game content matches.'), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-base-game-content-browser-retry')),
    );
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('Asghan'), findsOneWidget);
  });

  testWidgets('loaded browser is overflow-safe at 280 by 300', (tester) async {
    tester.view.physicalSize = const Size(280, 300);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await _pump(tester, loader: (_) async => catalog);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-base-game-content-browser-results')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-base-game-content-browser-search')),
      findsOneWidget,
    );
  });
}

Future<void> _pump(
  WidgetTester tester, {
  String? gameRoot = _gameRoot,
  Object sourceIdentity = 'generation-a',
  required Revision3BaseGameContentCatalogLoader loader,
  VoidCallback? openSettings,
  ValueChanged<String>? createNpcDraft,
  ValueChanged<String>? createQuestDraft,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3BaseGameContentBrowser(
        gameRoot: gameRoot,
        sourceIdentity: sourceIdentity,
        loader: loader,
        copy: _copy,
        openSettings: openSettings,
        createNpcDraft: createNpcDraft ?? (_) {},
        createQuestDraft: createQuestDraft ?? (_) {},
      ),
    ),
  ),
);

class _MutableHarness extends StatefulWidget {
  const _MutableHarness({required this.loader, super.key});

  final Revision3BaseGameContentCatalogLoader loader;

  @override
  State<_MutableHarness> createState() => _MutableHarnessState();
}

class _MutableHarnessState extends State<_MutableHarness> {
  Object _sourceIdentity = 'generation-a';

  void changeSource(Object value) => setState(() => _sourceIdentity = value);

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3BaseGameContentBrowser(
        gameRoot: _gameRoot,
        sourceIdentity: _sourceIdentity,
        loader: widget.loader,
        copy: _copy,
        createNpcDraft: (_) {},
        createQuestDraft: (_) {},
      ),
    ),
  );
}

Revision3BaseGameContentCatalog _simpleCatalog(String npcName) =>
    Revision3BaseGameContentCatalog(
      npcs: Revision3NpcCatalog(
        choices: [
          Revision3NpcCatalogChoice(
            catalogId: 'test:npc:${npcName.toLowerCase()}',
            displayName: npcName,
          ),
        ],
      ),
      quests: Revision3QuestCatalog(
        parents: [
          Revision3QuestParentChoice(
            catalogId: 'test:quest:chapter',
            displayName: 'Chapter',
            runtimeClass: 'UQuest_Chapter',
          ),
        ],
        givers: [
          Revision3QuestGiverChoice(
            catalogId: 'test:npc:giver',
            displayName: 'Quest giver',
            runtimeUniqueName: 'QUEST_GIVER',
          ),
        ],
      ),
    );

Future<Revision3BaseGameContentCatalog> _catalog() async {
  final story = await _storySelections();
  final archetypes = await _archetypeCatalog(_archetypeRecords());
  final adapter = StoryCatalogAdapter.fromSelectionsAndArchetypes(
    story,
    archetypes,
  );
  return Revision3BaseGameContentCatalog(
    npcs: Revision3NpcCatalog.fromStoryCatalog(adapter),
    quests: Revision3QuestCatalog.fromStoryCatalog(
      adapter,
      catalogSeal: story.catalogSeal,
      generationExecutableSeal: story.generation.executable,
    ),
  );
}

List<Map<String, Object?>> _archetypeRecords() => <Map<String, Object?>>[
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
    spawn: _genericSpawn,
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
];

Future<AuthoringStoryCatalogSelections> _storySelections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      _storyReadCommand: _storySelectionsResponse(_catalogRequest),
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogRequest);

Map<String, Object?> _storySelectionsResponse(
  String catalogJson, {
  Map<String, Object?>? generation,
}) {
  final exactGeneration = generation ?? _generation();
  return <String, Object?>{
    'ok': true,
    'request_catalog_sha256': crypto.sha256
        .convert(utf8.encode(catalogJson))
        .toString(),
    'selections': <String, Object?>{
      'schema_revision': 1,
      'generation': _deepCopy(exactGeneration),
      'catalog_seal': _storyCatalogSeal(),
      'npcs': _storyNpcSelections(),
      'quest_parents': <Object?>[_questParent()],
      'quest_collision_catalog': <String, Object?>{
        'status': 'inventory_unavailable',
        'catalog_layer': 'resolved-loadout.scripts.v1',
        'source_seal': _deepCopy(exactGeneration['shipping_cache']!),
        'blocks_draft_creation': true,
      },
      'blocks_build': true,
    },
  };
}

List<Map<String, Object?>> _storyNpcSelections() => <Map<String, Object?>>[
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
];

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

Map<String, Object?> _questParent() => <String, Object?>{
  'catalog_id': _questId,
  'display_name': 'Chapter',
  'quest_class': _storyClass(
    _questId,
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

Future<AuthoringNpcArchetypeCatalogBuildResult> _archetypeCatalog(
  List<Map<String, Object?>> records,
) => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      _npcCommand: _archetypeCatalogResponse(records),
    },
  ),
).authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: _gameRoot);

Map<String, Object?> _archetypeCatalogResponse(
  List<Map<String, Object?>> records, {
  Map<String, Object?>? generation,
}) {
  final exactGeneration = generation ?? _generation();
  final sourceIdentity = <String, Object?>{
    'shipping_cache': _deepCopy(exactGeneration['shipping_cache']!),
    'binds_cache': _deepCopy(exactGeneration['binds_cache']!),
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
    'generation': _deepCopy(exactGeneration),
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
    'generation': _deepCopy(exactGeneration),
    'catalog_seal': _deepCopy(artifact['catalog_seal']!),
    'source': _deepCopy(source),
    'payload_seal': _deepCopy(catalog['payload_seal']!),
    'record_count': records.length,
    'rejection_count': 0,
    'qualification': _deepCopy(catalog['qualification']!),
  };
  return response;
}

FakeGoreCoreFfiService _baseGameCatalogCore({
  Map<String, Object?>? archetypeGeneration,
}) {
  final storyGeneration = _generation();
  final catalogJson = _storyCatalogArtifactJson(storyGeneration);
  return FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      _storyBuildCommand: _storyCatalogBuildResponse(
        catalogJson,
        storyGeneration,
      ),
      _storyReadCommand: _storySelectionsResponse(
        catalogJson,
        generation: storyGeneration,
      ),
      _npcCommand: _archetypeCatalogResponse(
        _archetypeRecords(),
        generation: archetypeGeneration,
      ),
    },
  );
}

String _storyCatalogArtifactJson(Map<String, Object?> generation) =>
    jsonEncode(<String, Object?>{
      'format': 'story_catalog',
      'schema_revision': 1,
      'catalog': <String, Object?>{
        'generation': _deepCopy(generation),
        'record_set_id': 'g1r-steam-test-story-v1',
        'record_set_seal': _fixedSeal('e', 512),
        'npcs': _storyNpcSelections(),
        'quest_parents': <Object?>[_questParent()],
      },
      'catalog_seal': _storyCatalogSeal(),
    });

Map<String, Object?> _storyCatalogBuildResponse(
  String catalogJson,
  Map<String, Object?> generation,
) => <String, Object?>{
  'ok': true,
  'request_binding_sha256': _storyRequestBinding(_gameRoot),
  'catalog_json': catalogJson,
  'generation': _deepCopy(generation),
  'catalog_seal': _storyCatalogSeal(),
};

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

String _storyRequestBinding(String root) =>
    _rootRequestBinding(_storyBuildBindingDomain, root);

String _requestBinding(String root) {
  return _rootRequestBinding(_npcBindingDomain, root);
}

String _rootRequestBinding(String domain, String root) {
  final bytes = utf8.encode(root);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(domain),
    ...length,
    ...bytes,
  ]).toString();
}

String _hex(String digit) => List<String>.filled(64, digit).join();

Object _deepCopy(Object value) => jsonDecode(jsonEncode(value))!;
