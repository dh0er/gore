import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_project_dashboard.dart';

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';

const _projectRootA = r'C:\managed\dashboard-a';
const _projectRootB = r'C:\managed\dashboard-b';
const _headA = '{"checkpoint":"dashboard-a"}';
const _headB = '{"checkpoint":"dashboard-b"}';
const _npcId = '77777777777777777777777777777777';
const _npcModuleId = '7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f';
const _questId = '88888888888888888888888888888888';
const _questModuleId = '99999999999999999999999999999999';
const _itemId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _extraLocalizationA = '10101010101010101010101010101010';
const _extraLocalizationB = '11111111111111111111111111111110';
const _extraVoiceTakeA = '56565656565656565656565656565656';
const _extraVoiceTakeB = '57575757575757575757575757575757';
const _technicalVoiceSlotId = '66666666666666666666666666666665';
const _selectedVoiceTakeId = '55000000000000000000000000000000';
const _assetSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _otherAssetSha =
    'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
const _stageManifestSha =
    'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd';
const _questCollisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';
const _stageManifestMediaType =
    'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1';
const _targetExecutableSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

const _copy = Revision3ProjectDashboardCopy(
  untitledProjectLabel: 'Untitled fixture project',
  draftStatusLabel: 'Draft workspace',
  projectVersionLabel: 'Project version',
  projectAuthorLabel: 'Project author',
  notProvidedLabel: 'Not provided',
  contentCountsHeading: 'My mod / Changes',
  changesDescription: 'Exact current authoring content.',
  npcDraftCountLabel: 'NPC drafts',
  questDraftCountLabel: 'Quest drafts',
  dialogLineCountLabel: 'Dialog lines',
  voiceTakeCountLabel: 'Voice',
  assetCountLabel: 'DataAssets',
  itemPatchLabel: 'Item edits',
  localizationEntryLabel: 'Text',
  voiceSlotLabel: 'Voice slot',
  generatedScriptLabel: 'Generated script',
  selectedVoiceTakeLabel: 'Selected take',
  technicalContentLabel: 'Technical content',
  technicalContentDescription: 'Generated helpers needing exact ownership.',
  emptyChangesTitle: 'No authored changes yet',
  emptyChangesDescription: 'Create Story, text, items, or DataAsset changes.',
  openChangeLabel: 'Open exact change',
  changeActionFailedMessage: 'The exact change could not be opened.',
  unresolvedReferenceCountLabel: 'Unresolved references',
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
  testWidgets(
    'shows semantic author groups in stable order with collapsed Technical',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1280, 1600));
      final index = _fixture(
        existingSlotHasSelectedTake: true,
        technicalSlotHasProblem: true,
      );
      final stages = _matchingDataAssetStages(index);
      await _pumpDashboard(
        tester,
        load: () async => index,
        loadDataAssetStages: () async => stages,
      );
      await tester.pumpAndSettle();

      expect(find.text('Dashboard fixture'), findsOneWidget);
      expect(find.text('1.2.3'), findsOneWidget);
      expect(find.text('Dashboard author'), findsOneWidget);
      expect(find.text(_copy.draftStatusLabel), findsOneWidget);

      final orderedGroups = <Finder>[
        _group('quests'),
        _group('npcs'),
        _group('items'),
        _group('dataAssets'),
        _group('dialog'),
        _group('text'),
        _group('voice'),
        find.byKey(const Key('revision3-project-dashboard-technical')),
      ];
      for (final group in orderedGroups) {
        expect(group, findsOneWidget);
      }
      for (var index = 1; index < orderedGroups.length; index++) {
        expect(
          tester.getTopLeft(orderedGroups[index - 1]).dy,
          lessThan(tester.getTopLeft(orderedGroups[index]).dy),
        );
      }

      expect(_change(_questId), findsOneWidget);
      expect(_change(_npcId), findsOneWidget);
      expect(_change(_itemId), findsOneWidget);
      expect(_change(revision3VoiceContentLineId), findsOneWidget);
      expect(_change(_extraLocalizationA), findsOneWidget);
      expect(_change(_extraLocalizationB), findsOneWidget);
      expect(_change(_extraVoiceTakeA), findsOneWidget);
      expect(_change(_extraVoiceTakeB), findsOneWidget);

      expect(
        tester.getTopLeft(_change(_extraLocalizationB)).dy,
        lessThan(tester.getTopLeft(_change(_extraLocalizationA)).dy),
      );
      expect(
        tester.getTopLeft(_change(_extraVoiceTakeB)).dy,
        lessThan(tester.getTopLeft(_change(_extraVoiceTakeA)).dy),
      );

      final dataAssetRow = _change(stages.single.targetPath);
      expect(dataAssetRow, findsOneWidget);
      expect(
        tester.getSemantics(dataAssetRow).label,
        '${stages.single.targetPath.split('/').last}. '
        '${_copy.assetCountLabel}. ${stages.single.targetPath}',
      );
      expect(
        find.descendant(
          of: _group('dataAssets'),
          matching: find.byType(ListTile),
        ),
        findsOneWidget,
      );
      expect(_change(_stageManifestSha), findsNothing);
      expect(
        tester.getSemantics(_change(_selectedVoiceTakeId)).label,
        contains(_copy.selectedVoiceTakeLabel),
      );

      await tester.ensureVisible(
        find.byKey(
          const Key('revision3-project-dashboard-technical-expansion'),
        ),
      );
      await tester.pumpAndSettle();
      final technicalExpansion = tester.widget<ExpansionTile>(
        find.byKey(
          const Key('revision3-project-dashboard-technical-expansion'),
        ),
      );
      expect(
        (technicalExpansion.title as Text).data,
        '${_copy.technicalContentLabel} (1)',
      );
      expect(_change(_technicalVoiceSlotId), findsNothing);
      await tester.tap(
        find.byKey(
          const Key('revision3-project-dashboard-technical-expansion'),
        ),
      );
      await tester.pumpAndSettle();
      expect(_change(_technicalVoiceSlotId), findsOneWidget);
      expect(
        tester.getSemantics(_change(_technicalVoiceSlotId)).label,
        contains('1 ${_copy.unresolvedReferenceCountLabel}'),
      );

      expect(
        find.byKey(const Key('revision3-project-dashboard-missing-game')),
        findsNothing,
      );
    },
  );

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
                find.byKey(const Key('revision3-project-dashboard-changes')),
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

  testWidgets('routes exact entity, ItemPatch, and DataAsset action objects', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1280, 1600));
    final index = _fixture();
    final stages = _matchingDataAssetStages(index);
    Revision3ContentEntity? openedEntity;
    String? openedVanillaClass;
    AuthoringRevision3DataAssetStage? openedStage;

    await _pumpDashboard(
      tester,
      load: () async => index,
      loadDataAssetStages: () async => stages,
      changeActions: Revision3ProjectDashboardChangeActions(
        openEntity: (entity) {
          if (entity.id == _technicalVoiceSlotId) {
            throw StateError(r'C:\private\technical-helper.json');
          }
          openedEntity = entity;
        },
        openItemPatch: (vanillaClass) {
          openedVanillaClass = vanillaClass;
        },
        openDataAsset: (stage) {
          openedStage = stage;
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.ensureVisible(_change(_questId));
    await tester.tap(_change(_questId));
    await tester.pumpAndSettle();
    expect(openedEntity, same(index.entityById(_questId)));

    await tester.ensureVisible(_change(_itemId));
    await tester.tap(_change(_itemId));
    await tester.pumpAndSettle();
    expect(openedVanillaClass, 'UItemDefinition_Apple');

    await tester.ensureVisible(_change(stages.single.targetPath));
    await tester.tap(_change(stages.single.targetPath));
    await tester.pumpAndSettle();
    expect(openedStage, same(stages.single));

    await tester.ensureVisible(
      find.byKey(const Key('revision3-project-dashboard-technical-expansion')),
    );
    await tester.tap(
      find.byKey(const Key('revision3-project-dashboard-technical-expansion')),
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(_change(_technicalVoiceSlotId));
    await tester.tap(_change(_technicalVoiceSlotId));
    await tester.pumpAndSettle();
    expect(find.text(_copy.changeActionFailedMessage), findsOneWidget);
    expect(find.textContaining('private'), findsNothing);
    expect(find.textContaining('technical-helper'), findsNothing);
  });

  testWidgets('shows a deliberate empty state for an exact empty project', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    await _pumpDashboard(tester, load: () async => _emptyFixture());
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-dashboard-changes-empty')),
      findsOneWidget,
    );
    expect(find.text(_copy.emptyChangesTitle), findsOneWidget);
    expect(find.text(_copy.emptyChangesDescription), findsOneWidget);
    for (final group in const <String>[
      'quests',
      'npcs',
      'items',
      'dataAssets',
      'dialog',
      'text',
      'voice',
    ]) {
      expect(_group(group), findsNothing);
    }
    expect(
      find.byKey(const Key('revision3-project-dashboard-technical')),
      findsNothing,
    );
  });

  testWidgets('exact-index builders switch empty Story copy after reload', (
    tester,
  ) async {
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
                projectRoot: _projectRootA,
                projectId: revision3VoiceContentProjectId,
                projectRevision: requestedRevision,
                projectHeadCanonicalJson: '$_headA-$requestedRevision',
                load: () async => _fixture(
                  revision: requestedRevision,
                  includeStoryDrafts: requestedStory,
                ),
                loadDataAssetStages: () async =>
                    const <AuthoringRevision3DataAssetStage>[],
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
    expect(find.text('Create your first character or quest.'), findsOneWidget);
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
    expect(find.text('No reference problems'), findsOneWidget);
    expect(
      find.text('This exact project index has no unresolved references.'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('sanitizes failed exact sources and retries both loaders', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var contentCalls = 0;
    var dataAssetCalls = 0;
    await _pumpDashboard(
      tester,
      load: () async {
        contentCalls++;
        if (contentCalls == 1) {
          return _fixture(
            revision: 8,
            projectName: r'C:\private\wrong-project',
          );
        }
        return _fixture();
      },
      loadDataAssetStages: () async {
        dataAssetCalls++;
        if (dataAssetCalls == 1) {
          throw StateError(r'C:\private\stage-receipt.json');
        }
        return const <AuthoringRevision3DataAssetStage>[];
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
    expect(find.textContaining('stage-receipt'), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-project-dashboard-retry')),
    );
    await tester.pumpAndSettle();

    expect(contentCalls, 2);
    expect(dataAssetCalls, 2);
    expect(
      find.byKey(const Key('revision3-project-dashboard-error')),
      findsNothing,
    );
    expect(find.text('Dashboard fixture'), findsOneWidget);
  });

  for (final changedField in const <String>['head', 'root']) {
    testWidgets(
      'reloads the same revision for changed $changedField and ignores stale async completion',
      (tester) async {
        await _setSurfaceSize(tester, const Size(900, 700));
        final contentLoads = <Completer<Revision3ContentIndex>>[
          Completer<Revision3ContentIndex>(),
          Completer<Revision3ContentIndex>(),
        ];
        final dataAssetLoads =
            <Completer<List<AuthoringRevision3DataAssetStage>>>[
              Completer<List<AuthoringRevision3DataAssetStage>>(),
              Completer<List<AuthoringRevision3DataAssetStage>>(),
            ];
        var contentCalls = 0;
        var dataAssetCalls = 0;
        var projectRoot = _projectRootA;
        var projectHead = _headA;
        late StateSetter rebuild;

        await tester.pumpWidget(
          MaterialApp(
            home: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Scaffold(
                  body: Revision3ProjectDashboard(
                    projectRoot: projectRoot,
                    projectId: revision3VoiceContentProjectId,
                    projectRevision: 7,
                    projectHeadCanonicalJson: projectHead,
                    load: () => contentLoads[contentCalls++].future,
                    loadDataAssetStages: () =>
                        dataAssetLoads[dataAssetCalls++].future,
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

        rebuild(() {
          if (changedField == 'head') {
            projectHead = _headB;
          } else {
            projectRoot = _projectRootB;
          }
        });
        await tester.pump();
        expect(contentCalls, 2);
        expect(dataAssetCalls, 2);

        contentLoads[1].complete(_fixture(projectName: 'Newest checkpoint'));
        dataAssetLoads[1].complete(const <AuthoringRevision3DataAssetStage>[]);
        await tester.pumpAndSettle();
        expect(find.text('Newest checkpoint'), findsOneWidget);

        contentLoads[0].complete(_fixture(projectName: 'Stale checkpoint'));
        dataAssetLoads[0].complete(const <AuthoringRevision3DataAssetStage>[]);
        await tester.pumpAndSettle();
        expect(find.text('Newest checkpoint'), findsOneWidget);
        expect(find.text('Stale checkpoint'), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('does not overflow at 360x640 with a 2.0 text scaler', (
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
      textScaler: const TextScaler.linear(2),
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
  Revision3ProjectDashboardDataAssetLoader? loadDataAssetStages,
  String projectRoot = _projectRootA,
  String projectHeadCanonicalJson = _headA,
  bool gameConfigured = true,
  List<Revision3ProjectDashboardAction> tasks = const [],
  Revision3ProjectDashboardChangeActions changeActions =
      const Revision3ProjectDashboardChangeActions(),
  Revision3ProjectDashboardAction? settingsAction,
  TextScaler? textScaler,
}) {
  final dashboard = Scaffold(
    body: Revision3ProjectDashboard(
      projectRoot: projectRoot,
      projectId: revision3VoiceContentProjectId,
      projectRevision: 7,
      projectHeadCanonicalJson: projectHeadCanonicalJson,
      load: load,
      loadDataAssetStages:
          loadDataAssetStages ??
          () async => const <AuthoringRevision3DataAssetStage>[],
      gameConfigured: gameConfigured,
      copy: _copy,
      tasks: tasks,
      changeActions: changeActions,
      settingsAction: settingsAction,
    ),
  );
  return tester.pumpWidget(
    MaterialApp(
      builder: textScaler == null
          ? null
          : (context, child) => MediaQuery(
              data: MediaQuery.of(context).copyWith(textScaler: textScaler),
              child: child!,
            ),
      home: dashboard,
    ),
  );
}

Finder _group(String name) =>
    find.byKey(Key('revision3-project-dashboard-change-group-$name'));

Finder _change(String stableId) =>
    find.byKey(Key('revision3-project-dashboard-change-$stableId'));

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

Map<String, Object?> _ownerReference({
  required String role,
  required String ownerId,
  required String ownerKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': ownerId,
    'expected_kind': ownerKind,
  },
  'resolution': 'resolved',
};

Revision3ContentIndex _fixture({
  int revision = 7,
  String projectName = 'Dashboard fixture',
  bool includeStoryDrafts = true,
  bool existingSlotHasSelectedTake = false,
  bool technicalSlotHasProblem = false,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 2,
    existingSlotHasSelectedTake: existingSlotHasSelectedTake,
  );
  json['project_name'] = projectName;
  json['project_version'] = '1.2.3';
  json['project_author'] = 'Dashboard author';

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
            'module_namespace': 'PROJECT.NPCS.FIXTURE_GUARD',
            'module_relative_path': 'Project/Npcs/FixtureGuard.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          _ownerReference(
            role: 'origin_owner',
            ownerId: _npcId,
            ownerKind: 'npc_draft',
          ),
          _ownerReference(
            role: 'script_owner',
            ownerId: _npcId,
            ownerKind: 'npc_draft',
          ),
        ],
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
            'objective_slots': <Object?>[1, 2],
            'transcript_count': 0,
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
              'entity_id': _questModuleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[
          <String, Object?>{
            'role': 'quest_collision_artifact',
            'sha256': _otherAssetSha,
            'byte_len': 200,
            'logical_name': null,
            'expected_media_type': _questCollisionMediaType,
            'resolution': 'resolved',
          },
        ],
      },
      <String, Object?>{
        'id': _questModuleId,
        'kind': 'script_module',
        'display_name': 'Fixture quest source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'owner': <String, Object?>{
            'project_id': revision3VoiceContentProjectId,
            'entity_id': _questId,
            'expected_kind': 'quest_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': 4,
            'module_namespace': 'PROJECT.QUESTS.FIXTURE_QUEST',
            'module_relative_path': 'Project/Quests/FixtureQuest.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          _ownerReference(
            role: 'origin_owner',
            ownerId: _questId,
            ownerKind: 'quest_draft',
          ),
          _ownerReference(
            role: 'script_owner',
            ownerId: _questId,
            ownerKind: 'quest_draft',
          ),
        ],
        'asset_references': <Object?>[],
      },
    ]);
  }
  entities.addAll(<Object?>[
    _localizationEntity(
      id: _extraLocalizationA,
      displayName: 'Zulu text',
      locId: 'DIA_ZULU_TEXT',
    ),
    _localizationEntity(
      id: _extraLocalizationB,
      displayName: 'alpha text',
      locId: 'DIA_ALPHA_TEXT',
    ),
    _voiceTakeEntity(id: _extraVoiceTakeA, displayName: 'Zulu take'),
    _voiceTakeEntity(id: _extraVoiceTakeB, displayName: 'alpha take'),
    _technicalVoiceSlotEntity(hasProblem: technicalSlotHasProblem),
    _itemPatchEntity(),
  ]);
  entities.sort(
    (left, right) => ((left! as Map<String, Object?>)['id']! as String)
        .compareTo((right! as Map<String, Object?>)['id']! as String),
  );
  json['entities'] = entities;
  json['entity_counts'] = <String, Object?>{
    'localization_entry': 3,
    'dialog_line': 1,
    'voice_slot': 2,
    'voice_take': 4,
    if (includeStoryDrafts) 'npc_draft': 1,
    if (includeStoryDrafts) 'quest_draft': 1,
    if (includeStoryDrafts) 'script_module': 2,
    'item_patch': 1,
  };
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
      'media_type': _questCollisionMediaType,
      'class': 'quest_collision_artifact',
    },
    <String, Object?>{
      'sha256': _stageManifestSha,
      'byte_len': 300,
      'media_type': _stageManifestMediaType,
      'class': 'data_asset_stage_manifest',
    },
  ];
  return Revision3ContentIndex.fromJsonObject(json);
}

Map<String, Object?> _localizationEntity({
  required String id,
  required String displayName,
  required String locId,
}) => <String, Object?>{
  'id': id,
  'kind': 'localization_entry',
  'display_name': displayName,
  'revision': 0,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': locId},
  'summary': <String, Object?>{
    'kind': 'localization_entry',
    'data': <String, Object?>{'loc_id': locId, 'locales': <Object?>[]},
  },
  'references': <Object?>[],
  'asset_references': <Object?>[],
};

Map<String, Object?> _voiceTakeEntity({
  required String id,
  required String displayName,
}) => <String, Object?>{
  'id': id,
  'kind': 'voice_take',
  'display_name': displayName,
  'revision': 0,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': 'TAKE_$id'},
  'summary': <String, Object?>{
    'kind': 'voice_take',
    'data': <String, Object?>{
      'locale': 'de',
      'status': 'recorded',
      'codec': 'vorbis',
      'channels': 1,
      'sample_rate': 48000,
    },
  },
  'references': <Object?>[],
  'asset_references': <Object?>[],
};

Map<String, Object?> _technicalVoiceSlotEntity({bool hasProblem = false}) =>
    <String, Object?>{
      'id': _technicalVoiceSlotId,
      'kind': 'voice_slot',
      'display_name': 'Detached generated helper',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.voice-slot',
        'generator_version': 1,
        'owner': <String, Object?>{
          'project_id': revision3VoiceContentProjectId,
          'entity_id': revision3VoiceContentLineId,
          'expected_kind': 'dialog_line',
        },
      },
      'summary': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': 'en',
          'target_resolution': 'unresolved',
          'candidate_count': hasProblem ? 1 : 0,
          'has_selected_take': false,
        },
      },
      'references': <Object?>[
        _ownerReference(
          role: 'origin_owner',
          ownerId: revision3VoiceContentLineId,
          ownerKind: 'dialog_line',
        ),
        if (hasProblem)
          <String, Object?>{
            'role': 'voice_candidate',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': revision3VoiceContentProjectId,
              'entity_id': 'ffffffffffffffffffffffffffffffff',
              'expected_kind': 'voice_take',
            },
            'resolution': 'missing_entity',
          },
      ],
      'asset_references': <Object?>[],
    };

Map<String, Object?> _itemPatchEntity() => <String, Object?>{
  'id': _itemId,
  'kind': 'item_patch',
  'display_name': 'Fixture apple',
  'revision': 1,
  'origin': <String, Object?>{
    'type': 'vanilla',
    'generation': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256': _targetExecutableSha,
      },
    },
    'catalog_layer': 'base-game.g1r.items.v1',
    'canonical_selector': 'UItemDefinition_Apple',
    'source_seal': <String, Object?>{
      'byte_len': 456,
      'sha256':
          'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
    },
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
};

Revision3ContentIndex _emptyFixture() =>
    Revision3ContentIndex.fromJsonObject(<String, Object?>{
      'schema_revision': 1,
      'project_id': revision3VoiceContentProjectId,
      'project_revision': 7,
      'project_name': 'Empty dashboard fixture',
      'project_version': '1.0.0',
      'project_author': 'Dashboard author',
      'target': <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 171698176,
          'sha256': _targetExecutableSha,
        },
      },
      'authoring_locales': <Object?>[],
      'entity_counts': <String, Object?>{},
      'entities': <Object?>[],
      'assets': <Object?>[],
    });

List<AuthoringRevision3DataAssetStage> _matchingDataAssetStages(
  Revision3ContentIndex index,
) {
  final basisProjectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': index.projectId,
    'revision': index.projectRevision - 1,
    'meta': <String, Object?>{
      'name': index.projectName,
      'version': index.projectVersion,
      'author': index.projectAuthor,
    },
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': index.targetExecutableByteLength,
        'sha256': index.targetExecutableSha256,
      },
    },
    'authoring_locales': index.authoringLocales,
    'entities': <String, Object?>{},
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
  final fixture = Revision3DataAssetFixture.fromBasis(
    basisHead: revision3DataAssetHeadForProject(basisProjectJson),
    basisProjectJson: basisProjectJson,
  );
  return AuthoringRevision3DataAssetStageListResult.fromJson(
    fixture.listResponse(),
    expectedHead: fixture.stagedHead,
  ).stages;
}
