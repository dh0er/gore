import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_content_library.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _questId = '33333333333333333333333333333333';
const _moduleId = '44444444444444444444444444444444';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _assetSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _artifactSha =
    'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';

void main() {
  testWidgets('shows loading and exact-current content at desktop width', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final pending = Completer<Revision3ContentIndex>();

    await _pumpLibrary(tester, load: () => pending.future);

    expect(find.byKey(const Key('revision3-content-loading')), findsOneWidget);
    expect(find.text('Opening the exact current project...'), findsOneWidget);

    pending.complete(_fixture());
    await tester.pumpAndSettle();

    expect(find.text('Fixture project'), findsOneWidget);
    expect(find.text('3 entities / 2 assets / revision 7'), findsOneWidget);
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsOneWidget);
    expect(
      find.byKey(Key('revision3-content-entity-$_questId')),
      findsOneWidget,
    );
    expect(
      find.text(
        'Read-only exact project view. Build readiness has not been evaluated; runtime behavior is unqualified.',
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsOneWidget,
    );
    expect(find.text('GORE_GATE_GUARD'), findsWidgets);
  });

  testWidgets('disables Edit Quest without an edit lease', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();

    final editQuest = find.byKey(Key('revision3-content-edit-quest-$_questId'));
    expect(editQuest, findsOneWidget);
    await tester.tap(editQuest);
    await tester.pumpAndSettle();

    expect(find.text('Name & objectives'), findsNothing);
    expect(find.text('Description & connections'), findsNothing);
    expect(find.text('States & transitions'), findsNothing);
    expect(
      find.byKey(Key('revision3-content-edit-quest-outline-$_questId')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-content-edit-quest-context-$_questId')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-content-edit-quest-transitions-$_questId')),
      findsNothing,
    );
  });

  testWidgets('Edit Quest keeps outline usable when context has no game root', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var outlineCalls = 0;
    await _pumpLoadedLibrary(
      tester,
      editQuestOutline: (index, quest) async => outlineCalls++,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();

    await tester.tap(find.byKey(Key('revision3-content-edit-quest-$_questId')));
    await tester.pumpAndSettle();

    expect(find.text('Name & objectives'), findsOneWidget);
    expect(find.text('Description & connections'), findsOneWidget);
    expect(find.text('States & transitions'), findsOneWidget);
    expect(find.text('Configure the game installation first'), findsOneWidget);
    expect(
      tester
          .widget<PopupMenuItem<Object?>>(
            find.byKey(Key('revision3-content-edit-quest-context-$_questId')),
          )
          .enabled,
      isFalse,
    );
    await tester.tap(find.text('Name & objectives'));
    await tester.pumpAndSettle();
    expect(outlineCalls, 1);
  });

  testWidgets('Edit Quest routes states and transitions separately', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var transitionCalls = 0;
    await _pumpLoadedLibrary(
      tester,
      editQuestTransitions: (index, quest) async => transitionCalls++,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await tester.tap(find.byKey(Key('revision3-content-edit-quest-$_questId')));
    await tester.pumpAndSettle();

    await tester.tap(find.text('States & transitions'));
    await tester.pumpAndSettle();
    expect(transitionCalls, 1);
  });

  testWidgets(
    'V4 Quest disables legacy outline while context and transitions stay available',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var outlineCalls = 0;
      var contextCalls = 0;
      await _pumpLoadedLibrary(
        tester,
        questGeneratorVersion: 4,
        editQuestOutline: (index, quest) async => outlineCalls++,
        editQuestContext: (index, quest) async => contextCalls++,
        editQuestTransitions: (index, quest) async {},
      );
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pump();
      await tester.tap(
        find.byKey(Key('revision3-content-edit-quest-$_questId')),
      );
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<PopupMenuItem<Object?>>(
              find.byKey(Key('revision3-content-edit-quest-outline-$_questId')),
            )
            .enabled,
        isFalse,
      );
      expect(
        find.text('Not available yet after adding custom behavior'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<PopupMenuItem<Object?>>(
              find.byKey(Key('revision3-content-edit-quest-context-$_questId')),
            )
            .enabled,
        isTrue,
      );
      expect(
        tester
            .widget<PopupMenuItem<Object?>>(
              find.byKey(
                Key('revision3-content-edit-quest-transitions-$_questId'),
              ),
            )
            .enabled,
        isTrue,
      );

      await tester.tap(find.text('Description & connections'));
      await tester.pumpAndSettle();
      expect(outlineCalls, 0);
      expect(contextCalls, 1);
    },
  );

  testWidgets('Edit Quest routes description and connections separately', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var contextCalls = 0;
    await _pumpLoadedLibrary(
      tester,
      editQuestOutline: (index, quest) async {},
      editQuestContext: (index, quest) async => contextCalls++,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await tester.tap(find.byKey(Key('revision3-content-edit-quest-$_questId')));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Description & connections'));
    await tester.pumpAndSettle();
    expect(contextCalls, 1);
  });

  testWidgets('searches content and filters by semantic kind', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'characterdefinition_asghan',
    );
    await tester.pump();

    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsOneWidget);
    expect(find.byKey(Key('revision3-content-entity-$_questId')), findsNothing);

    await tester.tap(find.byTooltip('Clear search'));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'Report the secured gate',
    );
    await tester.pump();
    expect(
      find.byKey(Key('revision3-content-entity-$_questId')),
      findsOneWidget,
    );
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsNothing);

    await tester.tap(find.byTooltip('Clear search'));
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-content-filter-quest_draft')),
    );
    await tester.pump();

    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsNothing);
    expect(
      find.byKey(Key('revision3-content-entity-$_questId')),
      findsOneWidget,
    );
    expect(find.text('Find Homer'), findsWidgets);
  });

  testWidgets('switches to searchable read-only assets', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.tap(find.byKey(const Key('revision3-content-mode-assets')));
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-content-asset-list')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-content-asset-$_assetSha')),
      findsOneWidget,
    );
    expect(find.text('Voice audio'), findsWidgets);
    expect(find.text('audio/ogg'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-content-asset-details')),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'missing asset',
    );
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-content-asset-empty')),
      findsOneWidget,
    );
  });

  testWidgets('navigates outgoing references and derived backlinks', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.tap(
      find.byKey(const Key('revision3-content-filter-quest_draft')),
    );
    await tester.pump();
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();

    await tester.drag(
      find.byKey(const Key('revision3-content-entity-details')),
      const Offset(0, -250),
    );
    await tester.pump();
    await tester.tap(find.text('draft script module'));
    await tester.pump();
    expect(
      find.byKey(Key('revision3-content-entity-$_moduleId')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('revision3-content-entity-details-$_moduleId')),
      findsOneWidget,
    );

    await tester.drag(
      find.byKey(const Key('revision3-content-entity-details')),
      const Offset(0, -500),
    );
    await tester.pump();
    await tester.tap(
      find.byKey(
        const Key(
          'revision3-content-backlink-$_moduleId-$_questId-draft_script_module-0',
        ),
      ),
    );
    await tester.pump();
    expect(
      find.byKey(const ValueKey('revision3-content-entity-details-$_questId')),
      findsOneWidget,
    );

    await tester.drag(
      find.byKey(const Key('revision3-content-entity-details')),
      const Offset(0, -250),
    );
    await tester.pump();
    await tester.tap(find.text('quest collision artifact'));
    await tester.pump();
    expect(
      find.byKey(Key('revision3-content-asset-$_artifactSha')),
      findsOneWidget,
    );
    expect(
      find.byKey(
        const ValueKey('revision3-content-asset-details-$_artifactSha'),
      ),
      findsOneWidget,
    );

    await tester.drag(
      find.byKey(const Key('revision3-content-asset-details')),
      const Offset(0, -350),
    );
    await tester.pump();
    await tester.tap(
      find.byKey(
        const Key(
          'revision3-content-asset-backlink-$_artifactSha-$_questId-quest_collision_artifact-0',
        ),
      ),
    );
    await tester.pump();
    expect(
      find.byKey(const ValueKey('revision3-content-entity-details-$_questId')),
      findsOneWidget,
    );
  });

  testWidgets('opens details from the compact one-pane layout', (tester) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    await _pumpLoadedLibrary(tester);

    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );

    await tester.tap(find.byKey(Key('revision3-content-entity-$_npcId')));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsOneWidget,
    );
    expect(find.text('Stable ID'), findsOneWidget);
    expect(find.text(_npcId), findsOneWidget);
  });

  testWidgets('compact Quest sheet closes before each editor callback', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    var transitionCalls = 0;
    var sheetWasVisibleAtCallback = false;
    await _pumpLoadedLibrary(
      tester,
      editQuestTransitions: (index, quest) async {
        transitionCalls++;
        sheetWasVisibleAtCallback = find
            .byKey(const Key('revision3-content-entity-details'))
            .evaluate()
            .isNotEmpty;
      },
    );

    Future<void> openTransitions() async {
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-content-entity-details')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(Key('revision3-content-edit-quest-$_questId')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('States & transitions'));
      await tester.pumpAndSettle();
    }

    await openTransitions();
    expect(transitionCalls, 1);
    expect(sheetWasVisibleAtCallback, isFalse);
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );

    await openTransitions();
    expect(transitionCalls, 2);
    expect(sheetWasVisibleAtCallback, isFalse);
  });

  testWidgets('keeps same-role qualified backlinks as distinct siblings', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLibrary(
      tester,
      load: () async => _fixture(duplicateBacklinks: true),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await tester.drag(
      find.byKey(const Key('revision3-content-entity-details')),
      const Offset(0, -650),
    );
    await tester.pump();

    expect(
      find.byKey(
        const Key(
          'revision3-content-backlink-$_questId-$_moduleId-origin_owner-0',
        ),
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(
        const Key(
          'revision3-content-backlink-$_questId-$_moduleId-origin_owner-2',
        ),
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('shows a friendly error and retries the exact reopen', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var calls = 0;
    await _pumpLibrary(
      tester,
      load: () {
        calls += 1;
        if (calls == 1) return Future.error(StateError('fixture offline'));
        return Future.value(_fixture());
      },
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-content-error')), findsOneWidget);
    expect(find.textContaining('fixture offline'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-content-retry')));
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.byKey(const Key('revision3-content-error')), findsNothing);
    expect(find.text('Fixture project'), findsOneWidget);
  });

  testWidgets('rejects an index from another project checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    await _pumpLibrary(
      tester,
      projectRevision: 8,
      load: () async => _fixture(),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-content-error')), findsOneWidget);
    expect(
      find.text('Content index does not match the current project checkpoint.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-content-entity-list')),
      findsNothing,
    );
  });

  testWidgets('ignores loader closure identity but reloads a new checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var calls = 0;
    var revision = 7;
    var root = 'managed-root-a';
    var head = 'canonical-head-a';
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3ContentLibrary(
                projectRoot: root,
                projectId: _projectId,
                projectRevision: revision,
                projectHeadCanonicalJson: head,
                load: () async {
                  calls += 1;
                  return _fixture(revision: revision);
                },
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(calls, 1);

    rebuild(() {});
    await tester.pumpAndSettle();
    expect(calls, 1);

    rebuild(() => head = 'canonical-head-b');
    await tester.pumpAndSettle();
    expect(calls, 2, reason: 'same ID/revision but a new exact head reloads');

    rebuild(() => root = 'managed-root-b');
    await tester.pumpAndSettle();
    expect(
      calls,
      3,
      reason: 'same ID/revision/head under another root reloads',
    );

    rebuild(() => revision = 8);
    await tester.pumpAndSettle();
    expect(calls, 4);
    expect(find.text('3 entities / 2 assets / revision 8'), findsOneWidget);
  });
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pumpLoadedLibrary(
  WidgetTester tester, {
  int questGeneratorVersion = 2,
  Revision3QuestOutlineEditor? editQuestOutline,
  Revision3QuestContextEditor? editQuestContext,
  Revision3QuestTransitionsEditor? editQuestTransitions,
}) async {
  await _pumpLibrary(
    tester,
    load: () async => _fixture(questGeneratorVersion: questGeneratorVersion),
    editQuestOutline: editQuestOutline,
    editQuestContext: editQuestContext,
    editQuestTransitions: editQuestTransitions,
  );
  await tester.pumpAndSettle();
}

Future<void> _pumpLibrary(
  WidgetTester tester, {
  required Revision3ContentIndexLoader load,
  int projectRevision = 7,
  Revision3QuestOutlineEditor? editQuestOutline,
  Revision3QuestContextEditor? editQuestContext,
  Revision3QuestTransitionsEditor? editQuestTransitions,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3ContentLibrary(
        projectRoot: 'managed-root',
        projectId: _projectId,
        projectRevision: projectRevision,
        projectHeadCanonicalJson: 'canonical-head-$projectRevision',
        load: load,
        editQuestOutline: editQuestOutline,
        editQuestContext: editQuestContext,
        editQuestTransitions: editQuestTransitions,
      ),
    ),
  ),
);

Revision3ContentIndex _fixture({
  int revision = 7,
  bool duplicateBacklinks = false,
  int questGeneratorVersion = 2,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': revision,
  'project_name': 'Fixture project',
  'project_version': '0.1.0',
  'project_author': 'GORE',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 123, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{
    'npc_draft': 1,
    'quest_draft': 1,
    'script_module': 1,
  },
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
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _questId,
      'kind': 'quest_draft',
      'display_name': 'Find Homer',
      'revision': 1,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_FIND_HOMER',
      },
      'summary': <String, Object?>{
        'kind': 'quest_draft',
        'data': <String, Object?>{
          'technical_id': 'GORE_FIND_HOMER',
          'title': 'Find Homer',
          'objective_title': 'Ask Asghan about Homer',
          'additional_objective_titles': <String>[
            'Inspect the old gate',
            'Report the secured gate',
          ],
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'parent_runtime_class': 'B_Quest_FindHomer_C',
          'giver_runtime_unique_name': 'ASGHAN',
        },
      },
      'references': <Object?>[
        <String, Object?>{
          'role': 'draft_script_module',
          'qualifier': null,
          'target': <String, Object?>{
            'project_id': _projectId,
            'entity_id': _moduleId,
            'expected_kind': 'script_module',
          },
          'resolution': 'resolved',
        },
      ],
      'asset_references': <Object?>[
        <String, Object?>{
          'role': 'quest_collision_artifact',
          'sha256': _artifactSha,
          'byte_len': 8192,
          'logical_name': null,
          'expected_media_type':
              'application/vnd.gore.quest-collision-capability+json;version=2',
          'resolution': 'resolved',
        },
      ],
    },
    <String, Object?>{
      'id': _moduleId,
      'kind': 'script_module',
      'display_name': 'Find Homer source',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.quest-draft',
        'generator_version': questGeneratorVersion,
        'owner': <String, Object?>{
          'project_id': _projectId,
          'entity_id': _questId,
          'expected_kind': 'quest_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.quest-draft',
          'generator_version': questGeneratorVersion,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'Project/Quests/FindHomer.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        <String, Object?>{
          'role': 'origin_owner',
          'qualifier': null,
          'target': <String, Object?>{
            'project_id': _projectId,
            'entity_id': _questId,
            'expected_kind': 'quest_draft',
          },
          'resolution': 'resolved',
        },
        <String, Object?>{
          'role': 'script_owner',
          'qualifier': null,
          'target': <String, Object?>{
            'project_id': _projectId,
            'entity_id': _questId,
            'expected_kind': 'quest_draft',
          },
          'resolution': 'resolved',
        },
        if (duplicateBacklinks)
          <String, Object?>{
            'role': 'origin_owner',
            'qualifier': 'alternate',
            'target': <String, Object?>{
              'project_id': _projectId,
              'entity_id': _questId,
              'expected_kind': 'quest_draft',
            },
            'resolution': 'resolved',
          },
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[
    <String, Object?>{
      'sha256': _assetSha,
      'byte_len': 4096,
      'media_type': 'audio/ogg',
      'class': 'voice_audio',
    },
    <String, Object?>{
      'sha256': _artifactSha,
      'byte_len': 8192,
      'media_type':
          'application/vnd.gore.quest-collision-capability+json;version=2',
      'class': 'quest_collision_artifact',
    },
  ],
});
