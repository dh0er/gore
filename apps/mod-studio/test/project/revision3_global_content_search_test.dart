import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_base_game_content_browser.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_global_content_search.dart';
import 'package:gore_mod/project/revision3_global_content_search_view.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';

void main() {
  test('empty query is idle and performs no native loads', () async {
    var thisModLoads = 0;
    var baseLoads = 0;
    var installedLoads = 0;
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () async {
        thisModLoads++;
        return _contentIndex(const <String>['Never']);
      },
      loadBaseGame: () async {
        baseLoads++;
        return _baseCatalog();
      },
      loadInstalled: () async {
        installedLoads++;
        return _packageIndex(const <String>['/Game/Never']);
      },
    );
    addTearDown(controller.dispose);

    await controller.search('   ');

    expect((thisModLoads, baseLoads, installedLoads), (0, 0, 0));
    expect(controller.snapshot.query, isEmpty);
    expect(
      controller.snapshot.thisMod.phase,
      Revision3GlobalContentSourcePhase.idle,
    );
  });

  test(
    'explicit search starts all sources independently before completion',
    () async {
      final thisMod = Completer<Revision3ContentIndex>();
      final base = Completer<Revision3BaseGameContentCatalog>();
      final installed =
          Completer<AuthoringRevision3DataAssetPackageIndexResult>();
      var calls = 0;
      final controller = Revision3GlobalContentSearchController(
        loadThisMod: () {
          calls++;
          return thisMod.future;
        },
        loadBaseGame: () {
          calls++;
          return base.future;
        },
        loadInstalled: () {
          calls++;
          return installed.future;
        },
      );
      addTearDown(controller.dispose);

      final pending = controller.search('match');
      expect(calls, 3);
      expect(
        controller.snapshot.baseGame.phase,
        Revision3GlobalContentSourcePhase.loading,
      );

      thisMod.complete(_contentIndex(const <String>['Match entity']));
      base.complete(_baseCatalog(npcNames: const <String>['Match NPC']));
      installed.complete(_packageIndex(const <String>['/Game/DA_Match']));
      await pending;

      expect(controller.snapshot.thisMod.results, hasLength(1));
      expect(controller.snapshot.baseGame.results, hasLength(1));
      expect(controller.snapshot.installed.results, hasLength(1));
    },
  );

  test('errors and partial metadata remain isolated per source', () async {
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () async => _contentIndex(const <String>['Match entity']),
      loadBaseGame: () async => throw StateError('base unavailable'),
      loadInstalled: () async =>
          _packageIndex(const <String>['/Game/DA_Match'], partial: true),
    );
    addTearDown(controller.dispose);

    await controller.search('match');

    expect(
      controller.snapshot.thisMod.phase,
      Revision3GlobalContentSourcePhase.complete,
    );
    expect(
      controller.snapshot.baseGame.phase,
      Revision3GlobalContentSourcePhase.error,
    );
    expect(
      controller.snapshot.installed.phase,
      Revision3GlobalContentSourcePhase.partial,
    );
    expect(controller.snapshot.installed.truncated, isFalse);
  });

  test(
    'retry reloads only the failed source and retains successful rows',
    () async {
      var thisModLoads = 0;
      var baseLoads = 0;
      var installedLoads = 0;
      final controller = Revision3GlobalContentSearchController(
        loadThisMod: () async {
          thisModLoads++;
          return _contentIndex(const <String>['Match entity']);
        },
        loadBaseGame: () async {
          baseLoads++;
          if (baseLoads == 1) throw StateError('temporary base failure');
          return _baseCatalog(npcNames: const <String>['Match NPC']);
        },
        loadInstalled: () async {
          installedLoads++;
          return _packageIndex(const <String>['/Game/DA_Match']);
        },
      );
      addTearDown(controller.dispose);

      await controller.search('match');
      final retainedThisMod = controller.snapshot.thisMod.results.single;
      final retainedInstalled = controller.snapshot.installed.results.single;

      await controller.retrySource(Revision3GlobalContentSource.baseGame);

      expect((thisModLoads, baseLoads, installedLoads), (1, 2, 1));
      expect(controller.snapshot.thisMod.results.single, same(retainedThisMod));
      expect(
        controller.snapshot.installed.results.single,
        same(retainedInstalled),
      );
      expect(controller.snapshot.baseGame.results.single.title, 'Match NPC');
    },
  );

  test('every source caps allocation at 100 and reports truncation', () async {
    final names = List<String>.generate(101, (index) => 'Match $index');
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () async => _contentIndex(names),
      loadBaseGame: () async => _baseCatalog(npcNames: names),
      loadInstalled: () async => _packageIndex(<String>[
        for (var index = 0; index < 101; index++)
          '/Game/Generated/DA_Match_${index.toString().padLeft(3, '0')}',
      ]),
    );
    addTearDown(controller.dispose);

    await controller.search('match');

    for (final source in Revision3GlobalContentSource.values) {
      final state = controller.snapshot.stateFor(source);
      expect(state.results, hasLength(100), reason: source.name);
      expect(state.truncated, isTrue, reason: source.name);
      expect(
        state.phase,
        Revision3GlobalContentSourcePhase.partial,
        reason: source.name,
      );
    }
  });

  test('late epoch cannot replace newer results', () async {
    final oldThisMod = Completer<Revision3ContentIndex>();
    final oldBase = Completer<Revision3BaseGameContentCatalog>();
    final oldInstalled =
        Completer<AuthoringRevision3DataAssetPackageIndexResult>();
    final newThisMod = Completer<Revision3ContentIndex>();
    final newBase = Completer<Revision3BaseGameContentCatalog>();
    final newInstalled =
        Completer<AuthoringRevision3DataAssetPackageIndexResult>();
    var calls = 0;
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () => calls++ == 0 ? oldThisMod.future : newThisMod.future,
      loadBaseGame: () => calls++ == 1 ? oldBase.future : newBase.future,
      loadInstalled: () =>
          calls++ == 2 ? oldInstalled.future : newInstalled.future,
    );
    addTearDown(controller.dispose);

    final oldSearch = controller.search('result');
    final newSearch = controller.search('result');
    newThisMod.complete(_contentIndex(const <String>['New result']));
    newBase.complete(_baseCatalog(npcNames: const <String>['New result']));
    newInstalled.complete(_packageIndex(const <String>['/Game/NewResult']));
    await newSearch;
    oldThisMod.complete(_contentIndex(const <String>['Old result']));
    oldBase.complete(_baseCatalog(npcNames: const <String>['Old result']));
    oldInstalled.complete(_packageIndex(const <String>['/Game/OldResult']));
    await oldSearch;

    expect(controller.snapshot.thisMod.results.single.title, 'New result');
    expect(controller.snapshot.baseGame.results.single.title, 'New result');
    expect(controller.snapshot.installed.results.single.title, 'NewResult');
  });

  test('search is multi-term, case and common-accent tolerant', () async {
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () async => _contentIndex(const <String>['Górnik Hero']),
      loadBaseGame: () async => _baseCatalog(),
      loadInstalled: () async => _packageIndex(const <String>['/Game/Other']),
    );
    addTearDown(controller.dispose);

    await controller.search('GORNIK hero');

    expect(controller.snapshot.thisMod.results.single.title, 'Górnik Hero');
  });

  test('result actions retain exact same-source identities', () async {
    const path = '/Game/Characters/DA_Asghan';
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () async =>
          _contentIndex(const <String>['Asghan local'], includeAsset: true),
      loadBaseGame: () async => _baseCatalog(
        npcNames: const <String>['Asghan base'],
        questName: 'Asghan quest',
      ),
      loadInstalled: () async => _packageIndex(const <String>[path]),
    );
    addTearDown(controller.dispose);

    await controller.search('asghan');

    final local = controller.snapshot.thisMod.results.single;
    final base = controller.snapshot.baseGame.results;
    final installed = controller.snapshot.installed.results.single;
    expect(
      local.action!.kind,
      Revision3GlobalContentActionKind.openThisModEntity,
    );
    expect(local.action!.identity, local.subtitle);
    expect(
      base.map((result) => result.action!.kind),
      containsAll(<Revision3GlobalContentActionKind>[
        Revision3GlobalContentActionKind.createBaseNpcDraft,
        Revision3GlobalContentActionKind.createBaseQuestDraft,
      ]),
    );
    expect(installed.action!.identity, path);
    expect(installed.subtitle, path);

    await controller.search('APPLICATION');
    final asset = controller.snapshot.thisMod.results.single;
    expect(asset.kind, Revision3GlobalContentKind.thisModAsset);
    expect(
      asset.action!.kind,
      Revision3GlobalContentActionKind.openThisModAsset,
    );
    expect(asset.action!.identity, 'b' * 64);
  });

  test(
    'atomic source update retains Base but drops changed exact sources',
    () async {
      final identity = Revision3GlobalContentSearchSourceIdentity(
        project: 'project-a',
        thisMod: 'revision-1',
        baseGame: 'game-root-a',
        installed: 'mount-1',
      );
      final controller = Revision3GlobalContentSearchController(
        sourceIdentity: identity,
        loadThisMod: () async => _contentIndex(const <String>['Match local']),
        loadBaseGame: () async => _baseCatalog(npcNames: const ['Match base']),
        loadInstalled: () async => _packageIndex(const ['/Game/Match']),
      );
      addTearDown(controller.dispose);
      await controller.search('match');

      controller.updateSources(
        sourceIdentity: const Revision3GlobalContentSearchSourceIdentity(
          project: 'project-a',
          thisMod: 'revision-2',
          baseGame: 'game-root-a',
          installed: 'mount-2',
        ),
        loadThisMod: () async => _contentIndex(const <String>['Fresh local']),
        loadBaseGame: () async => _baseCatalog(npcNames: const ['Match base']),
        loadInstalled: () async => _packageIndex(const ['/Game/Fresh']),
      );

      expect(controller.snapshot.query, 'match');
      expect(
        controller.snapshot.thisMod.phase,
        Revision3GlobalContentSourcePhase.idle,
      );
      expect(controller.snapshot.baseGame.results.single.title, 'Match base');
      expect(
        controller.snapshot.installed.phase,
        Revision3GlobalContentSourcePhase.idle,
      );

      controller.updateSources(
        sourceIdentity: const Revision3GlobalContentSearchSourceIdentity(
          project: 'project-a',
          thisMod: 'revision-2',
          baseGame: 'game-root-b',
          installed: 'mount-2',
        ),
        loadThisMod: () async => _contentIndex(const <String>['Fresh local']),
        loadBaseGame: () async => _baseCatalog(npcNames: const ['Fresh base']),
        loadInstalled: () async => _packageIndex(const ['/Game/Fresh']),
      );
      expect(
        controller.snapshot.baseGame.phase,
        Revision3GlobalContentSourcePhase.idle,
      );

      controller.updateSources(
        sourceIdentity: const Revision3GlobalContentSearchSourceIdentity(
          project: 'project-b',
          thisMod: 'revision-0',
          baseGame: 'game-root-b',
          installed: 'mount-2',
        ),
        loadThisMod: () async => _contentIndex(const <String>['Other']),
        loadBaseGame: () async => _baseCatalog(),
        loadInstalled: () async => _packageIndex(const ['/Game/Other']),
      );
      expect(controller.snapshot.query, isEmpty);
      expect(
        controller.snapshot.baseGame.phase,
        Revision3GlobalContentSourcePhase.idle,
      );
    },
  );

  testWidgets('submit stays disabled until superseded physical reads settle', (
    tester,
  ) async {
    final thisMod = Completer<Revision3ContentIndex>();
    final base = Completer<Revision3BaseGameContentCatalog>();
    final installed =
        Completer<AuthoringRevision3DataAssetPackageIndexResult>();
    var loads = 0;
    final controller = Revision3GlobalContentSearchController(
      loadThisMod: () {
        loads++;
        return thisMod.future;
      },
      loadBaseGame: () {
        loads++;
        return base.future;
      },
      loadInstalled: () {
        loads++;
        return installed.future;
      },
    );
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Revision3GlobalContentSearchView(
            controller: controller,
            copy: _copy(),
            callbacks: Revision3GlobalContentSearchCallbacks(
              openThisModEntity: (_) {},
              openThisModAsset: (_) {},
              createBaseNpcDraft: (_) {},
              createBaseQuestDraft: (_) {},
              inspectInstalledDataAsset: (_) {},
            ),
          ),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key('revision3-global-content-search-field')),
      'match',
    );
    await tester.tap(
      find.byKey(const Key('revision3-global-content-search-submit')),
    );
    await tester.pump();

    expect(loads, 3);
    expect(controller.isLoading, isTrue);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-global-content-search-submit')),
          )
          .onPressed,
      isNull,
    );

    await tester.tap(
      find.byKey(const Key('revision3-global-content-search-clear')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-global-content-search-field')),
      'new match',
    );
    await tester.pump();
    expect(controller.snapshot.query, isEmpty);
    expect(controller.isLoading, isTrue);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-global-content-search-submit')),
          )
          .onPressed,
      isNull,
    );
    expect(loads, 3);

    thisMod.complete(_contentIndex(const <String>['Match entity']));
    base.complete(_baseCatalog(npcNames: const <String>['Match NPC']));
    installed.complete(_packageIndex(const <String>['/Game/DA_Match']));
    await tester.pumpAndSettle();

    expect(controller.isLoading, isFalse);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-global-content-search-submit')),
          )
          .onPressed,
      isNotNull,
    );
    expect(loads, 3);

    await tester.tap(
      find.byKey(const Key('revision3-global-content-search-submit')),
    );
    await tester.pumpAndSettle();
    expect(loads, 6);
  });

  testWidgets(
    'host focus node targets the query without starting source loads',
    (tester) async {
      var loads = 0;
      final controller = Revision3GlobalContentSearchController(
        loadThisMod: () async {
          loads++;
          return _contentIndex(const <String>[]);
        },
        loadBaseGame: () async {
          loads++;
          return _baseCatalog();
        },
        loadInstalled: () async {
          loads++;
          return _packageIndex(const <String>[]);
        },
      );
      final focusNode = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(focusNode.dispose);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Revision3GlobalContentSearchView(
              controller: controller,
              copy: _copy(),
              callbacks: Revision3GlobalContentSearchCallbacks(
                openThisModEntity: (_) {},
                openThisModAsset: (_) {},
                createBaseNpcDraft: (_) {},
                createBaseQuestDraft: (_) {},
                inspectInstalledDataAsset: (_) {},
              ),
              queryFocusNode: focusNode,
            ),
          ),
        ),
      );

      focusNode.requestFocus();
      await tester.pump();

      expect(focusNode.hasFocus, isTrue);
      expect(loads, 0);
    },
  );

  testWidgets(
    'narrow layout searches explicitly and dispatches exact actions',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(320, 700));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      var loads = 0;
      final openedEntities = <String>[];
      final createdNpcs = <String>[];
      final inspectedPaths = <String>[];
      const installedPath = '/Game/Characters/DA_Match';
      final controller = Revision3GlobalContentSearchController(
        loadThisMod: () async {
          loads++;
          return _contentIndex(const <String>['Match entity']);
        },
        loadBaseGame: () async {
          loads++;
          return _baseCatalog(npcNames: const <String>['Match NPC']);
        },
        loadInstalled: () async {
          loads++;
          return _packageIndex(const <String>[installedPath]);
        },
      );
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Revision3GlobalContentSearchView(
              controller: controller,
              copy: _copy(),
              callbacks: Revision3GlobalContentSearchCallbacks(
                openThisModEntity: openedEntities.add,
                openThisModAsset: (_) {},
                createBaseNpcDraft: createdNpcs.add,
                createBaseQuestDraft: (_) {},
                inspectInstalledDataAsset: inspectedPaths.add,
              ),
            ),
          ),
        ),
      );

      expect(loads, 0);
      expect(
        find.byKey(const Key('revision3-global-content-search-empty-prompt')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-global-content-search-field')),
        'match',
      );
      await tester.tap(
        find.byKey(const Key('revision3-global-content-search-submit')),
      );
      await tester.pumpAndSettle();

      expect(loads, 3);
      expect(tester.takeException(), isNull);
      final entityAction = find.byKey(
        const ValueKey<Object>((
          'global-search-action',
          Revision3GlobalContentActionKind.openThisModEntity,
          '00000000000000000000000000000001',
        )),
      );
      await tester.ensureVisible(entityAction);
      await tester.pumpAndSettle();
      await tester.tap(entityAction);
      final npcAction = find.byKey(
        const ValueKey<Object>((
          'global-search-action',
          Revision3GlobalContentActionKind.createBaseNpcDraft,
          'npc-0',
        )),
      );
      await tester.scrollUntilVisible(
        npcAction,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(npcAction);
      final installedAction = find.byKey(
        const ValueKey<Object>((
          'global-search-action',
          Revision3GlobalContentActionKind.inspectInstalledDataAsset,
          installedPath,
        )),
      );
      await tester.scrollUntilVisible(
        installedAction,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(installedAction);

      expect(openedEntities, const <String>[
        '00000000000000000000000000000001',
      ]);
      expect(createdNpcs, const <String>['npc-0']);
      expect(inspectedPaths, const <String>[installedPath]);
      expect(tester.takeException(), isNull);

      final clear = find.byKey(
        const Key('revision3-global-content-search-clear'),
      );
      await tester.scrollUntilVisible(
        clear,
        -500,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(clear);
      await tester.pump();
      expect(controller.snapshot.query, isEmpty);
      expect(loads, 3);
    },
  );
}

Revision3GlobalContentSearchCopy _copy() => Revision3GlobalContentSearchCopy(
  title: 'Search all content',
  searchLabel: 'Query',
  searchAction: 'Search',
  clearAction: 'Clear',
  emptyPrompt: 'Enter a query to load sources.',
  noResults: 'No results',
  loading: 'Loading',
  loadFailed: 'Load failed',
  retry: 'Retry',
  partial: 'Partial',
  complete: 'Complete',
  truncated: 'Only the first 100 results are shown.',
  openAction: 'Open',
  createDraftAction: 'Create draft',
  inspectAction: 'Inspect',
  sourceLabels: const <Revision3GlobalContentSource, String>{
    Revision3GlobalContentSource.thisMod: 'This mod',
    Revision3GlobalContentSource.baseGame: 'Base game',
    Revision3GlobalContentSource.installed: 'Installed',
  },
  kindLabels: const <Revision3GlobalContentKind, String>{
    Revision3GlobalContentKind.thisModEntity: 'Entity',
    Revision3GlobalContentKind.thisModAsset: 'Asset',
    Revision3GlobalContentKind.baseNpc: 'NPC',
    Revision3GlobalContentKind.baseQuest: 'Quest',
    Revision3GlobalContentKind.experimentalBaseNpc: 'NPC evidence',
    Revision3GlobalContentKind.installedDataAsset: 'DataAsset',
  },
  readinessLabels: const <Revision3GlobalContentReadiness, String>{
    Revision3GlobalContentReadiness.exactCurrent: 'Exact current',
    Revision3GlobalContentReadiness.exactCurrentWithProblems:
        'Exact with problems',
    Revision3GlobalContentReadiness.offlineDraftRuntimeUnqualified:
        'Offline draft',
    Revision3GlobalContentReadiness.inspectOnlyRuntimeUnqualified:
        'Inspect only',
    Revision3GlobalContentReadiness.metadataOnlyRuntimeUnqualified:
        'Metadata only',
  },
);

Revision3ContentIndex _contentIndex(
  List<String> names, {
  bool includeAsset = false,
}) {
  const projectId = '11111111111111111111111111111111';
  final entities = <Object?>[];
  for (var index = 0; index < names.length; index++) {
    final npcId = (index + 1).toRadixString(16).padLeft(32, '0');
    final moduleId = (0x10000 + index).toRadixString(16).padLeft(32, '0');
    final namespace = 'PROJECT.NPCS.NPC$index';
    entities.addAll(<Object?>[
      <String, Object?>{
        'id': npcId,
        'kind': 'npc_draft',
        'display_name': names[index],
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'NPC_$index',
        },
        'summary': <String, Object?>{
          'kind': 'npc_draft',
          'data': <String, Object?>{
            'unique_name': 'NPC_$index',
            'module_namespace': namespace,
            'parent_character_definition': 'CharacterDefinition_$index',
            'parent_ai_agent_config': 'AIAgentConfig_$index',
            'parent_spawn_definition': 'SpawnDefinition_$index',
            'greeting_count': 0,
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'draft_script_module',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': projectId,
              'entity_id': moduleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[],
      },
      <String, Object?>{
        'id': moduleId,
        'kind': 'script_module',
        'display_name': 'Generated character source $index',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'owner': <String, Object?>{
            'project_id': projectId,
            'entity_id': npcId,
            'expected_kind': 'npc_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.logical-npc-clone-draft',
            'generator_version': 1,
            'module_namespace': namespace,
            'module_relative_path': 'Project/Npcs/Npc$index.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          _storyOwnerReference(
            role: 'origin_owner',
            projectId: projectId,
            ownerId: npcId,
          ),
          _storyOwnerReference(
            role: 'script_owner',
            projectId: projectId,
            ownerId: npcId,
          ),
        ],
        'asset_references': <Object?>[],
      },
    ]);
  }
  entities.sort(
    (left, right) => ((left! as Map<String, Object?>)['id']! as String)
        .compareTo((right! as Map<String, Object?>)['id']! as String),
  );
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': projectId,
    'project_revision': 1,
    'project_name': 'Fixture',
    'project_version': '0.1.0',
    'project_author': 'GORE',
    'target': <String, Object?>{'executable': _seal(123, 'a' * 64)},
    'authoring_locales': <Object?>['de'],
    'entity_counts': names.isEmpty
        ? <String, Object?>{}
        : <String, Object?>{
            'npc_draft': names.length,
            'script_module': names.length,
          },
    'entities': entities,
    'assets': includeAsset
        ? <Object?>[
            <String, Object?>{
              'sha256': 'b' * 64,
              'byte_len': 5,
              'media_type': 'application/octet-stream',
              'class': 'other',
            },
          ]
        : <Object?>[],
  });
}

Map<String, Object?> _storyOwnerReference({
  required String role,
  required String projectId,
  required String ownerId,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': projectId,
    'entity_id': ownerId,
    'expected_kind': 'npc_draft',
  },
  'resolution': 'resolved',
};

Revision3BaseGameContentCatalog _baseCatalog({
  List<String> npcNames = const <String>['Other NPC'],
  String questName = 'Other quest',
}) => Revision3BaseGameContentCatalog(
  npcs: Revision3NpcCatalog(
    choices: <Revision3NpcCatalogChoice>[
      for (var index = 0; index < npcNames.length; index++)
        Revision3NpcCatalogChoice(
          catalogId: 'npc-$index',
          displayName: npcNames[index],
        ),
    ],
  ),
  quests: Revision3QuestCatalog(
    parents: <Revision3QuestParentChoice>[
      Revision3QuestParentChoice(
        catalogId: 'quest-parent',
        displayName: questName,
        runtimeClass: 'B_Quest_Fixture_C',
      ),
    ],
    givers: <Revision3QuestGiverChoice>[
      Revision3QuestGiverChoice(
        catalogId: 'quest-giver',
        displayName: 'Fixture giver',
        runtimeUniqueName: 'FIXTURE_GIVER',
      ),
    ],
  ),
);

AuthoringRevision3DataAssetPackageIndexResult _packageIndex(
  List<String> paths, {
  bool partial = false,
}) {
  final sorted = paths.toList()..sort();
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _seal(4096, 'c' * 64),
    }),
  );
  final candidates = <Object?>[
    for (var index = 0; index < sorted.length; index++)
      <String, Object?>{
        'target_path': sorted[index],
        'package_id_hex': index.toRadixString(16).padLeft(16, '0'),
      },
  ];
  final packageJson = jsonEncode(<String, Object?>{
    'status': partial ? 'partial_index' : 'complete_index',
    'physical_chunk_count': candidates.length + (partial ? 1 : 0),
    'winning_export_bundle_count': candidates.length + (partial ? 1 : 0),
    'directory_indexed_export_bundle_count': candidates.length,
    'out_of_scope_export_bundle_count': 0,
    'candidates': candidates,
    'partial_reasons': partial
        ? <Object?>[
            <String, Object?>{
              'reason': 'missing_directory_index_path',
              'count': 1,
            },
          ]
        : <Object?>[],
  });
  final bytes = utf8.encode(packageJson);
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': candidates.length,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': _seal(80, 'd' * 64),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': packageJson,
      'package_index_seal': _seal(
        bytes.length,
        crypto.sha256.convert(bytes).toString(),
      ),
      'package_index_status': partial ? 'partial_index' : 'complete_index',
      'project_id': '31313131313131313131313131313131',
      'project_revision': 7,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': _seal(120, 'e' * 64),
      'target_executable_seal': _seal(999, 'f' * 64),
    },
    expectedHead: head,
  );
}

Map<String, Object?> _seal(int bytes, String sha) => <String, Object?>{
  'byte_len': bytes,
  'sha256': sha,
};
