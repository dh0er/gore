import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_project_dashboard.dart';

import '../support/revision3_voice_content_fixture.dart';

const _npcId = '77777777777777777777777777777777';
const _questId = '88888888888888888888888888888888';
const _missingModuleId = '99999999999999999999999999999999';
const _assetSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _otherAssetSha =
    'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';

const _copy = Revision3ProjectDashboardCopy(
  untitledProjectLabel: 'Untitled fixture project',
  draftStatusLabel: 'Draft workspace',
  projectVersionLabel: 'Project version',
  projectAuthorLabel: 'Project author',
  notProvidedLabel: 'Not provided',
  contentCountsHeading: 'My mod content',
  npcDraftCountLabel: 'NPC drafts',
  questDraftCountLabel: 'Quest drafts',
  dialogLineCountLabel: 'Dialog lines',
  voiceTakeCountLabel: 'Voice takes',
  assetCountLabel: 'Project assets',
  unresolvedReferenceCountLabel: 'Unresolved references',
  readinessHeading: 'Current capability',
  offlineAuthoringTitle: 'Bounded offline authoring available',
  offlineAuthoringDescription: 'Supported edits stay inside this project.',
  generalBuildBlockedTitle: 'General project build blocked',
  generalBuildBlockedDescription: 'Only separately qualified output may build.',
  runtimeUnqualifiedTitle: 'Game runtime unqualified',
  runtimeUnqualifiedDescription: 'Saving a draft proves no game behavior.',
  referenceIntegrityTitle: 'Reference integrity only',
  referenceIntegrityDescription: 'This count covers exact project references.',
  missingGameTitle: 'Game installation not configured',
  missingGameDescription: 'Configure it for game-bound read-only evidence.',
  createHeading: 'Create content',
  toolsHeading: 'Voice and project tools',
  loadingSemanticsLabel: 'Loading exact project overview',
  loadErrorSemanticsLabel: 'Project overview unavailable',
  loadErrorTitle: 'Overview could not be opened',
  loadErrorDescription: 'Retry the exact current project.',
  retryLabel: 'Retry overview',
);

void main() {
  testWidgets('loads exact counts and reports only bounded readiness', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1280, 900));
    await _pumpDashboard(tester, load: () async => _fixture());
    await tester.pumpAndSettle();

    expect(find.text('Dashboard fixture'), findsOneWidget);
    expect(find.text('1.2.3'), findsOneWidget);
    expect(find.text('Dashboard author'), findsOneWidget);
    expect(find.text(_copy.draftStatusLabel), findsOneWidget);

    _expectCount(tester, 'npc-drafts', 1);
    _expectCount(tester, 'quest-drafts', 1);
    _expectCount(tester, 'dialog-lines', 1);
    _expectCount(tester, 'voice-takes', 2);
    _expectCount(tester, 'assets', 2);
    _expectCount(tester, 'unresolved-references', 1);
    expect(
      find.byKey(
        const Key('revision3-project-dashboard-reference-status-count'),
      ),
      findsOneWidget,
    );

    expect(find.text(_copy.offlineAuthoringTitle), findsOneWidget);
    expect(find.text(_copy.generalBuildBlockedTitle), findsOneWidget);
    expect(find.text(_copy.runtimeUnqualifiedTitle), findsOneWidget);
    expect(find.text(_copy.referenceIntegrityTitle), findsOneWidget);
    expect(find.text(_copy.generalBuildBlockedDescription), findsOneWidget);
    expect(find.text(_copy.runtimeUnqualifiedDescription), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-project-dashboard-missing-game')),
      findsNothing,
    );
  });

  testWidgets('routes enabled actions and keeps unavailable actions visible', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 900));
    var createCalls = 0;
    var toolCalls = 0;
    var settingsCalls = 0;
    final enabledCreate = Revision3ProjectDashboardAction(
      id: 'create-npc-draft',
      icon: Icons.person_add_alt_1_outlined,
      title: 'Create NPC draft',
      description: 'Create a bounded NPC draft.',
      onPressed: () => createCalls++,
    );
    const disabledCreate = Revision3ProjectDashboardAction(
      id: 'create-dialog',
      icon: Icons.add_comment_outlined,
      title: 'Create dialog',
      description: 'Dialog creation is not available.',
      onPressed: null,
    );
    final tool = Revision3ProjectDashboardAction(
      id: 'manage-voice-takes',
      icon: Icons.library_music_outlined,
      title: 'Manage Voice takes',
      description: 'Manage exact project candidates.',
      onPressed: () => toolCalls++,
    );
    final settings = Revision3ProjectDashboardAction(
      id: 'open-settings',
      icon: Icons.settings_outlined,
      title: 'Open settings',
      description: 'Configure the game installation.',
      onPressed: () => settingsCalls++,
    );

    await _pumpDashboard(
      tester,
      load: () async => _fixture(),
      gameConfigured: false,
      createActions: [enabledCreate, disabledCreate],
      toolActions: [tool],
      settingsAction: settings,
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-dashboard-missing-game')),
      findsOneWidget,
    );
    final enabled = find.byKey(
      const Key('revision3-project-dashboard-action-create-npc-draft'),
    );
    await tester.ensureVisible(enabled);
    await tester.tap(enabled);
    expect(createCalls, 1);

    final disabled = find.byKey(
      const Key('revision3-project-dashboard-action-create-dialog'),
    );
    expect(tester.widget<InkWell>(disabled).onTap, isNull);
    expect(find.text('Create dialog'), findsOneWidget);

    final toolFinder = find.byKey(
      const Key('revision3-project-dashboard-action-manage-voice-takes'),
    );
    await tester.ensureVisible(toolFinder);
    await tester.tap(toolFinder);
    expect(toolCalls, 1);

    final settingsFinder = find.byKey(
      const Key('revision3-project-dashboard-settings-action'),
    );
    await tester.ensureVisible(settingsFinder);
    await tester.tap(settingsFinder);
    expect(settingsCalls, 1);
  });

  testWidgets('sanitizes a mismatched index and retries exact content', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var calls = 0;
    await _pumpDashboard(
      tester,
      load: () async {
        calls++;
        if (calls == 1) {
          return _fixture(
            revision: 8,
            projectName: r'C:\private\wrong-project',
          );
        }
        return _fixture();
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-dashboard-error')),
      findsOneWidget,
    );
    expect(find.text(_copy.loadErrorTitle), findsOneWidget);
    expect(find.text(_copy.loadErrorDescription), findsOneWidget);
    expect(find.textContaining('private'), findsNothing);
    expect(find.textContaining('revision 8'), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-project-dashboard-retry')),
    );
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(
      find.byKey(const Key('revision3-project-dashboard-error')),
      findsNothing,
    );
    expect(find.text('Dashboard fixture'), findsOneWidget);
  });

  testWidgets('reloads a new revision and ignores stale async completion', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    final first = Completer<Revision3ContentIndex>();
    final second = Completer<Revision3ContentIndex>();
    var calls = 0;
    var revision = 7;
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3ProjectDashboard(
                projectId: revision3VoiceContentProjectId,
                projectRevision: revision,
                load: () => calls++ == 0 ? first.future : second.future,
                gameConfigured: true,
                copy: _copy,
                createActions: const [],
                toolActions: const [],
              ),
            );
          },
        ),
      ),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-project-dashboard-loading')),
      findsOneWidget,
    );

    rebuild(() => revision = 8);
    await tester.pump();
    expect(calls, 2);

    second.complete(_fixture(revision: 8, projectName: 'Newest checkpoint'));
    await tester.pumpAndSettle();
    expect(find.text('Newest checkpoint'), findsOneWidget);

    first.complete(_fixture(revision: 7, projectName: 'Stale checkpoint'));
    await tester.pumpAndSettle();
    expect(find.text('Newest checkpoint'), findsOneWidget);
    expect(find.text('Stale checkpoint'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('uses one-column cards without overflow at narrow width', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(320, 640));
    const create = Revision3ProjectDashboardAction(
      id: 'create-quest-draft',
      icon: Icons.assignment_add,
      title: 'Create a bounded Quest draft',
      description:
          'This deliberately long localized action description must wrap safely.',
      onPressed: null,
    );
    const tool = Revision3ProjectDashboardAction(
      id: 'build-voice-bundle',
      icon: Icons.inventory_2_outlined,
      title: 'Build a separate offline Voice bundle',
      description:
          'This is not a general project build, deployment, or runtime proof.',
      onPressed: null,
    );

    await _pumpDashboard(
      tester,
      load: () async => _fixture(),
      gameConfigured: false,
      createActions: const [create],
      toolActions: const [tool],
      settingsAction: const Revision3ProjectDashboardAction(
        id: 'open-settings',
        icon: Icons.settings_outlined,
        title: 'Open settings',
        description: 'Configure the game installation.',
        onPressed: null,
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    final scroll = find.byKey(const Key('revision3-project-dashboard-scroll'));
    await tester.drag(scroll, const Offset(0, -1800));
    await tester.pumpAndSettle();
    expect(
      find.byKey(
        const Key('revision3-project-dashboard-action-build-voice-bundle'),
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}

Future<void> _pumpDashboard(
  WidgetTester tester, {
  required Revision3ProjectDashboardLoader load,
  bool gameConfigured = true,
  List<Revision3ProjectDashboardAction> createActions = const [],
  List<Revision3ProjectDashboardAction> toolActions = const [],
  Revision3ProjectDashboardAction? settingsAction,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3ProjectDashboard(
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        load: load,
        gameConfigured: gameConfigured,
        copy: _copy,
        createActions: createActions,
        toolActions: toolActions,
        settingsAction: settingsAction,
      ),
    ),
  ),
);

void _expectCount(WidgetTester tester, String id, int value) {
  final tile = find.byKey(Key('revision3-project-dashboard-count-$id'));
  expect(tile, findsOneWidget);
  expect(
    find.descendant(of: tile, matching: find.text('$value')),
    findsOneWidget,
  );
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Revision3ContentIndex _fixture({
  int revision = 7,
  String projectName = 'Dashboard fixture',
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 2,
  );
  json['project_name'] = projectName;
  json['project_version'] = '1.2.3';
  json['project_author'] = 'Dashboard author';

  final counts = json['entity_counts']! as Map<String, Object?>;
  counts['npc_draft'] = 1;
  counts['quest_draft'] = 1;

  final entities = List<Object?>.from(json['entities']! as List<Object?>)
    ..addAll(<Object?>[
      <String, Object?>{
        'id': _npcId,
        'kind': 'npc_draft',
        'display_name': 'Fixture guard',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'FIXTURE_GUARD',
        },
        'summary': <String, Object?>{
          'kind': 'npc_draft',
          'data': <String, Object?>{
            'unique_name': 'FIXTURE_GUARD',
            'module_namespace': 'PROJECT.NPCS.FIXTURE_GUARD',
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
        'display_name': 'Fixture quest',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'FIXTURE_QUEST',
        },
        'summary': <String, Object?>{
          'kind': 'quest_draft',
          'data': <String, Object?>{
            'technical_id': 'FIXTURE_QUEST',
            'title': 'Fixture quest',
            'objective_title': 'Inspect the fixture',
            'additional_objective_titles': <String>['Report the fixture'],
            'module_namespace': 'PROJECT.QUESTS.FIXTURE_QUEST',
            'parent_runtime_class': 'B_Quest_FindHomer_C',
            'giver_runtime_unique_name': 'ASGHAN',
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'draft_script_module',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': revision3VoiceContentProjectId,
              'entity_id': _missingModuleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'missing_entity',
          },
        ],
        'asset_references': <Object?>[],
      },
    ])
    ..sort(
      (left, right) => ((left! as Map<String, Object?>)['id']! as String)
          .compareTo((right! as Map<String, Object?>)['id']! as String),
    );
  json['entities'] = entities;
  json['assets'] = <Object?>[
    <String, Object?>{
      'sha256': _assetSha,
      'byte_len': 100,
      'media_type': 'audio/ogg',
      'class': 'voice_audio',
    },
    <String, Object?>{
      'sha256': _otherAssetSha,
      'byte_len': 200,
      'media_type': 'application/octet-stream',
      'class': 'other',
    },
  ];
  return Revision3ContentIndex.fromJsonObject(json);
}
