import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_project_dashboard.dart';

import '../support/revision3_voice_content_fixture.dart';

const _npcId = '77777777777777777777777777777777';
const _npcModuleId = '7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f';
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
  continueHeading: 'Continue working',
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

  testWidgets(
    'routes full-width tasks with exact copy, semantics, and visible gates',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1000, 900));
      var storyCalls = 0;
      var contentCalls = 0;
      var gatedCalls = 0;
      var settingsCalls = 0;
      final story = Revision3ProjectDashboardAction(
        id: 'story',
        icon: Icons.menu_book_outlined,
        title: 'Open Story',
        description: 'Create or continue Story content.',
        titleBuilder: (index) =>
            'Continue ${_storyDraftCount(index)} Story drafts',
        descriptionBuilder: (index) =>
            '${_entityCountForTest(index, Revision3ContentEntityKind.questDraft)} quests and '
            '${_entityCountForTest(index, Revision3ContentEntityKind.npcDraft)} characters are ready to edit.',
        onPressed: () => storyCalls++,
      );
      const disabledContent = Revision3ProjectDashboardAction(
        id: 'content',
        icon: Icons.account_tree_outlined,
        title: 'Browse content',
        description: 'Browse all known project content.',
        onPressed: null,
      );
      final content = Revision3ProjectDashboardAction(
        id: 'localization-voice',
        icon: Icons.record_voice_over_outlined,
        title: 'Dialog and Voice',
        description: 'Continue project text and recordings.',
        controlKey: const Key('custom-localization-task'),
        onPressed: () => contentCalls++,
      );
      final gated = Revision3ProjectDashboardAction(
        id: 'build-release',
        icon: Icons.inventory_2_outlined,
        title: 'Build output',
        description: 'Review qualified output.',
        disabledReason:
            'No general project build is qualified for this checkpoint.',
        enabledFor: (_) => false,
        onPressed: () => gatedCalls++,
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
        tasks: [story, disabledContent, content, gated],
        settingsAction: settings,
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-project-dashboard-missing-game')),
        findsOneWidget,
      );
      expect(find.text(_copy.continueHeading), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-project-dashboard-tasks')),
        findsOneWidget,
      );
      expect(find.text('Continue 2 Story drafts'), findsOneWidget);
      expect(
        find.text('1 quests and 1 characters are ready to edit.'),
        findsOneWidget,
      );
      expect(
        tester
            .getTopLeft(
              find.byKey(const Key('revision3-project-dashboard-tasks')),
            )
            .dy,
        lessThan(
          tester
              .getTopLeft(
                find.byKey(const Key('revision3-project-dashboard-counts')),
              )
              .dy,
        ),
      );

      final storyFinder = find.byKey(
        const Key('revision3-project-dashboard-task-story'),
      );
      await tester.scrollUntilVisible(
        storyFinder,
        240,
        scrollable: find.descendant(
          of: find.byKey(const Key('revision3-project-dashboard-scroll')),
          matching: find.byType(Scrollable),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        tester.getSemantics(storyFinder),
        matchesSemantics(
          label: 'Continue 2 Story drafts',
          hint: '1 quests and 1 characters are ready to edit.',
          isButton: true,
          hasEnabledState: true,
          isEnabled: true,
        ),
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();
      expect(storyCalls, 1);

      final disabled = find.byKey(
        const Key('revision3-project-dashboard-task-content'),
      );
      expect(tester.widget<ListTile>(disabled).onTap, isNull);
      expect(find.text('Browse content'), findsOneWidget);

      final custom = find.byKey(const Key('custom-localization-task'));
      await tester.ensureVisible(custom);
      await tester.tap(custom);
      expect(contentCalls, 1);

      final gatedFinder = find.byKey(
        const Key('revision3-project-dashboard-task-build-release'),
      );
      await tester.ensureVisible(gatedFinder);
      expect(tester.widget<ListTile>(gatedFinder).onTap, isNull);
      expect(
        find.text('No general project build is qualified for this checkpoint.'),
        findsOneWidget,
      );
      expect(
        tester.getSemantics(gatedFinder),
        matchesSemantics(
          label: 'Build output',
          hint: 'No general project build is qualified for this checkpoint.',
          isButton: true,
          hasEnabledState: true,
          isEnabled: false,
        ),
      );
      await tester.tap(gatedFinder);
      expect(gatedCalls, 0);

      final settingsFinder = find.byKey(
        const Key('revision3-project-dashboard-settings-action'),
      );
      await tester.ensureVisible(settingsFinder);
      await tester.tap(settingsFinder);
      expect(settingsCalls, 1);
    },
  );

  testWidgets(
    'exact-index builders switch empty Story and problem copy after reload',
    (tester) async {
      await _setSurfaceSize(tester, const Size(900, 760));
      var revision = 7;
      var includeStoryDrafts = false;
      late StateSetter rebuild;
      final story = Revision3ProjectDashboardAction(
        id: 'story',
        icon: Icons.menu_book_outlined,
        title: 'Story fallback',
        description: 'Story fallback description.',
        titleBuilder: (index) {
          final count = _storyDraftCount(index);
          return count == 0 ? 'Start Story' : 'Continue $count Story drafts';
        },
        descriptionBuilder: (index) => _storyDraftCount(index) == 0
            ? 'Create your first character or quest.'
            : 'Return to the exact current Story workspace.',
        onPressed: () {},
      );
      final problems = Revision3ProjectDashboardAction(
        id: 'problems',
        icon: Icons.rule_folder_outlined,
        title: 'Problems fallback',
        description: 'Problems fallback description.',
        titleBuilder: (index) => index.problemCount == 0
            ? 'No reference problems'
            : 'Review ${index.problemCount} reference problem',
        descriptionBuilder: (index) => index.problemCount == 0
            ? 'This exact project index has no unresolved references.'
            : 'Open the exact blockers for this checkpoint.',
        onPressed: () {},
      );

      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              final requestedRevision = revision;
              final requestedStory = includeStoryDrafts;
              return Scaffold(
                body: Revision3ProjectDashboard(
                  projectId: revision3VoiceContentProjectId,
                  projectRevision: requestedRevision,
                  load: () async => _fixture(
                    revision: requestedRevision,
                    includeStoryDrafts: requestedStory,
                  ),
                  gameConfigured: true,
                  copy: _copy,
                  tasks: [story, problems],
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Start Story'), findsOneWidget);
      expect(
        find.text('Create your first character or quest.'),
        findsOneWidget,
      );
      expect(find.text('No reference problems'), findsOneWidget);
      expect(
        find.text('This exact project index has no unresolved references.'),
        findsOneWidget,
      );
      expect(find.text('Story fallback'), findsNothing);
      expect(find.text('Problems fallback'), findsNothing);

      rebuild(() {
        revision = 8;
        includeStoryDrafts = true;
      });
      await tester.pumpAndSettle();

      expect(find.text('Start Story'), findsNothing);
      expect(find.text('Continue 2 Story drafts'), findsOneWidget);
      expect(
        find.text('Return to the exact current Story workspace.'),
        findsOneWidget,
      );
      expect(find.text('Review 1 reference problem'), findsOneWidget);
      expect(
        find.text('Open the exact blockers for this checkpoint.'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

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
                tasks: const [],
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

  testWidgets('uses full-width task rows without overflow at 360x640', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    const story = Revision3ProjectDashboardAction(
      id: 'story',
      icon: Icons.assignment_add,
      title: 'Create a character or continue an existing Quest journey',
      description:
          'This deliberately long localized task description must wrap safely without becoming a separate card.',
      onPressed: null,
    );
    const build = Revision3ProjectDashboardAction(
      id: 'build-release',
      icon: Icons.inventory_2_outlined,
      title: 'Review the currently qualified build and release output',
      description:
          'This is not a general project build, deployment, or runtime proof.',
      disabledReason:
          'Finish the required validation before creating qualified output.',
      onPressed: null,
    );

    await _pumpDashboard(
      tester,
      load: () async => _fixture(),
      gameConfigured: false,
      tasks: const [story, build],
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
    final taskList = find.byKey(const Key('revision3-project-dashboard-tasks'));
    final storyRow = find.byKey(
      const Key('revision3-project-dashboard-task-story'),
    );
    final buildRow = find.byKey(
      const Key('revision3-project-dashboard-task-build-release'),
    );
    expect(taskList, findsOneWidget);
    expect(storyRow, findsOneWidget);
    expect(buildRow, findsOneWidget);
    expect(
      (tester.getSize(storyRow).width - tester.getSize(taskList).width).abs(),
      lessThan(0.01),
    );
    expect(
      (tester.getSize(buildRow).width - tester.getSize(taskList).width).abs(),
      lessThan(0.01),
    );
    expect(
      find.ancestor(of: storyRow, matching: find.byType(Card)),
      findsNothing,
    );
    expect(
      find.text(
        'Finish the required validation before creating qualified output.',
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
  List<Revision3ProjectDashboardAction> tasks = const [],
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
        tasks: tasks,
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

int _entityCountForTest(
  Revision3ContentIndex index,
  Revision3ContentEntityKind kind,
) => index.entities.where((entity) => entity.kind == kind).length;

int _storyDraftCount(Revision3ContentIndex index) =>
    _entityCountForTest(index, Revision3ContentEntityKind.npcDraft) +
    _entityCountForTest(index, Revision3ContentEntityKind.questDraft);

Revision3ContentIndex _fixture({
  int revision = 7,
  String projectName = 'Dashboard fixture',
  bool includeStoryDrafts = true,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 2,
  );
  json['project_name'] = projectName;
  json['project_version'] = '1.2.3';
  json['project_author'] = 'Dashboard author';

  final counts = json['entity_counts']! as Map<String, Object?>;
  if (includeStoryDrafts) {
    counts['npc_draft'] = 1;
    counts['quest_draft'] = 1;
    counts['script_module'] = 1;
  } else {
    counts.remove('npc_draft');
    counts.remove('quest_draft');
    counts.remove('script_module');
  }

  final entities = List<Object?>.from(json['entities']! as List<Object?>);
  if (includeStoryDrafts) {
    entities.addAll(<Object?>[
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
            'greeting_count': 0,
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'draft_script_module',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': revision3VoiceContentProjectId,
              'entity_id': _npcModuleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[],
      },
      <String, Object?>{
        'id': _npcModuleId,
        'kind': 'script_module',
        'display_name': 'Fixture guard source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'FIXTURE_GUARD_SOURCE',
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'dashboard.fixture.npc',
            'generator_version': 1,
            'module_namespace': 'PROJECT.NPCS.FIXTURE_GUARD',
            'module_relative_path': 'Project/Npcs/FixtureGuard.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
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
    ]);
  }
  entities.sort(
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
