import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_story_entity_workbench.dart';
import 'package:gore_mod/project/revision3_story_workspace.dart';

const _projectA = '11111111111111111111111111111111';
const _projectB = '99999999999999999999999999999999';
const _npcId = '22222222222222222222222222222222';
const _questId = '33333333333333333333333333333333';
const _moduleId = '44444444444444444444444444444444';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _artifactSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

final _copy = Revision3StoryWorkspaceCopy(
  title: 'Story',
  loadingLabel: 'Opening exact Story drafts',
  authorityNotice:
      'Project drafts only. Build and runtime readiness are not evaluated.',
  searchHint: 'Search NPCs and Quests',
  clearSearchLabel: 'Clear search',
  allFilterLabel: 'All',
  npcFilterLabel: 'NPCs',
  questFilterLabel: 'Quests',
  createNpcLabel: 'Create NPC',
  createQuestLabel: 'Create Quest',
  creatingNpcLabel: 'Creating NPC',
  creatingQuestLabel: 'Creating Quest',
  noStoryDrafts: 'No Story drafts yet',
  noMatchingStoryDrafts: 'No matching Story drafts',
  selectDraftLabel: 'Select an NPC or Quest',
  retryLabel: 'Retry',
  loadErrorTitle: 'Story could not be opened',
  checkpointMismatchError: 'Story index does not match this checkpoint.',
  checkpointSummary: (count, revision) => '$count drafts / revision $revision',
  loadErrorDetails: (error) => '$error',
  createErrorDetails: (error) => 'CREATE FAILED: $error',
  detailsSheetLabel: (name) => '$name details',
  workbench: const Revision3StoryEntityWorkbenchCopy.english(),
);

final _longGermanCopy = Revision3StoryWorkspaceCopy(
  title: 'Geschichten, Charaktere und umfangreiche Quest-Entwürfe',
  loadingLabel: 'Die exakten aktuellen Story-Entwürfe werden geöffnet',
  authorityNotice:
      'Hier werden ausschließlich Projektentwürfe bearbeitet. Die Build-Bereitschaft und das Verhalten zur Laufzeit wurden noch nicht geprüft.',
  searchHint:
      'NPCs und Quests nach Namen, technischer Kennung oder Beschreibung durchsuchen',
  clearSearchLabel: 'Suche vollständig zurücksetzen',
  allFilterLabel: 'Alle Entwürfe',
  npcFilterLabel: 'Nichtspielercharaktere',
  questFilterLabel: 'Quest-Entwürfe',
  createNpcLabel: 'Neuen Nichtspielercharakter erstellen',
  createQuestLabel: 'Neuen Quest-Entwurf erstellen',
  creatingNpcLabel: 'Nichtspielercharakter wird erstellt',
  creatingQuestLabel: 'Quest-Entwurf wird erstellt',
  noStoryDrafts: 'Noch keine Story-Entwürfe vorhanden',
  noMatchingStoryDrafts: 'Keine passenden Story-Entwürfe gefunden',
  selectDraftLabel: 'NPC oder Quest-Entwurf auswählen',
  retryLabel: 'Erneut versuchen',
  loadErrorTitle: 'Story-Arbeitsbereich konnte nicht geöffnet werden',
  checkpointMismatchError:
      'Der Story-Index gehört nicht zum exakten aktuellen Projektstand.',
  checkpointSummary: (count, revision) =>
      '$count Story-Entwürfe in Projektrevision $revision',
  loadErrorDetails: (error) => 'Ladefehler: $error',
  createErrorDetails: (error) => 'Erstellungsfehler: $error',
  detailsSheetLabel: (name) => 'Details für $name',
  workbench: const Revision3StoryEntityWorkbenchCopy.english(),
);

void main() {
  testWidgets(
    'loads only Story drafts with search, filters, and honest create actions',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final pending = Completer<Revision3ContentIndex>();

      await _pumpWorkspace(tester, load: () => pending.future);
      expect(
        find.byKey(const Key('revision3-story-workspace-loading')),
        findsOneWidget,
      );
      expect(find.text('Opening exact Story drafts'), findsOneWidget);

      pending.complete(_fixture());
      await tester.pumpAndSettle();

      expect(find.text('2 drafts / revision 7'), findsOneWidget);
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_moduleId')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-story-workspace-wide')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
        findsOneWidget,
      );

      expect(find.text('NPC creation is not configured.'), findsOneWidget);
      expect(find.text('Quest creation is not configured.'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .onPressed,
        isNull,
      );

      await tester.enterText(
        find.byKey(const Key('revision3-story-workspace-search')),
        'homer',
      );
      await tester.pump();
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsNothing,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-clear-search')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-filter-npc')),
      );
      await tester.pump();
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('enabled create callbacks remain direct and visible', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var npcCalls = 0;
    var questCalls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () async => npcCalls++,
      createQuestDraft: () async => questCalls++,
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-npc')),
    );
    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-quest')),
    );
    await tester.pumpAndSettle();

    expect(npcCalls, 1);
    expect(questCalls, 1);
    expect(find.text('NPC creation is not configured.'), findsNothing);
    expect(find.text('Quest creation is not configured.'), findsNothing);
  });

  testWidgets('NPC and Quest creation are mutually exclusive while pending', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final pendingNpc = Completer<void>();
    var npcCalls = 0;
    var questCalls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () {
        npcCalls++;
        return pendingNpc.future;
      },
      createQuestDraft: () async => questCalls++,
    );
    await tester.pumpAndSettle();

    final npc = find.byKey(const Key('revision3-story-workspace-create-npc'));
    final quest = find.byKey(
      const Key('revision3-story-workspace-create-quest'),
    );
    await tester.tap(npc);
    await tester.pump();

    expect(npcCalls, 1);
    expect(tester.widget<FilledButton>(npc).onPressed, isNull);
    expect(tester.widget<OutlinedButton>(quest).onPressed, isNull);
    await tester.tap(quest);
    await tester.pump();
    expect(questCalls, 0, reason: 'a second authoring dialog must not stack');

    pendingNpc.complete();
    await tester.pumpAndSettle();
    expect(tester.widget<FilledButton>(npc).onPressed, isNotNull);
    expect(tester.widget<OutlinedButton>(quest).onPressed, isNotNull);
    await tester.tap(quest);
    await tester.pumpAndSettle();
    expect(questCalls, 1);
  });

  testWidgets('create failures use their dedicated localized formatter', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () async => throw StateError('authoring unavailable'),
      createQuestDraft: () async {},
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-npc')),
    );
    await tester.pump();

    expect(
      find.textContaining('CREATE FAILED: Bad state: authoring unavailable'),
      findsOneWidget,
    );
    expect(find.textContaining('Story could not be opened'), findsNothing);
  });

  testWidgets(
    'retains exact selection and Workbench tab across a same-project revision',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var revision = 7;
      var index = _fixture();
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: _workspace(
                  revision: revision,
                  head: 'head-$revision',
                  load: () async => index,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();
      final logicTab = find.byKey(
        Key('revision3-story-workbench-tab-logic-$_questId'),
      );
      await tester.ensureVisible(logicTab);
      await tester.tap(logicTab);
      await tester.pump();
      expect(tester.widget<ChoiceChip>(logicTab).selected, isTrue);

      rebuild(() {
        revision = 8;
        index = _fixture(revision: 8);
      });
      await tester.pumpAndSettle();

      expect(
        find.byKey(
          ValueKey('revision3-story-workspace-workbench-$_projectA-$_questId'),
        ),
        findsOneWidget,
      );
      expect(tester.widget<ChoiceChip>(logicTab).selected, isTrue);

      rebuild(() {
        revision = 9;
        index = _fixture(revision: 9, includeQuest: false);
      });
      await tester.pumpAndSettle();
      expect(logicTab, findsNothing);
      expect(
        find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
        findsOneWidget,
      );

      rebuild(() {
        revision = 10;
        index = _fixture(revision: 10);
      });
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();
      final restoredLogic = find.byKey(
        Key('revision3-story-workbench-tab-logic-$_questId'),
      );
      expect(tester.widget<ChoiceChip>(restoredLogic).selected, isFalse);
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                Key('revision3-story-workbench-tab-overview-$_questId'),
              ),
            )
            .selected,
        isTrue,
      );
    },
  );

  testWidgets('project switch resets search, filter, selection, and tabs', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var projectId = _projectA;
    var root = 'root-a';
    var index = _fixture();
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                root: root,
                projectId: projectId,
                load: () async => index,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-filter-quest')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-story-workspace-search')),
      'homer',
    );
    await tester.tap(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
    );
    await tester.pump();
    final references = find.byKey(
      Key('revision3-story-workbench-tab-references-$_questId'),
    );
    await tester.ensureVisible(references);
    await tester.tap(references);
    await tester.pump();

    rebuild(() {
      projectId = _projectB;
      root = 'root-b';
      index = _fixture(
        projectId: _projectB,
        includeQuest: false,
        projectName: 'Other project',
      );
    });
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-story-workspace-search')),
          )
          .controller!
          .text,
      isEmpty,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(const Key('revision3-story-workspace-filter-all')),
          )
          .selected,
      isTrue,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
      findsOneWidget,
    );
  });

  testWidgets('360px and short 640x420 use list-to-details sheets', (
    tester,
  ) async {
    for (final size in const <Size>[Size(360, 760), Size(640, 420)]) {
      await tester.binding.setSurfaceSize(size);
      await _pumpWorkspace(tester, load: () async => _fixture());
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-wide')),
        findsNothing,
      );
      final quest = find.byKey(
        Key('revision3-story-workspace-entity-$_questId'),
      );
      await tester.scrollUntilVisible(
        quest,
        80,
        scrollable: find.descendant(
          of: find.byKey(const Key('revision3-story-workspace-list')),
          matching: find.byType(Scrollable),
        ),
      );
      await tester.pump();
      expect(quest.hitTestable(), findsOneWidget);
      await tester.tap(quest);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workbench-tab-overview-$_questId')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull, reason: 'viewport $size');

      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    }
    await tester.binding.setSurfaceSize(null);
  });

  testWidgets(
    'long German chrome stays usable and leaves a real list at tight height',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 420));
      const disabledReason =
          'Vor dem Erstellen muss eine vollständige Gothic-Spielinstallation ausgewählt und sicher geprüft werden.';
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(),
        copy: _longGermanCopy,
        createNpcDraftDisabledReason: disabledReason,
        createQuestDraftDisabledReason: disabledReason,
      );
      await tester.pumpAndSettle();

      final chrome = find.byKey(
        const Key('revision3-story-workspace-tight-chrome-scroll'),
      );
      final list = find.byKey(const Key('revision3-story-workspace-list'));
      expect(chrome, findsOneWidget);
      expect(list, findsOneWidget);
      expect(tester.getSize(list).height, greaterThanOrEqualTo(128));
      expect(
        find.byKey(const Key('revision3-story-workspace-authority-notice')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-story-workspace-create-npc-disabled-reason'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-story-workspace-create-quest-disabled-reason'),
        ),
        findsNothing,
        reason: 'one identical visible setup reason is sufficient',
      );
      expect(
        find
            .byKey(const Key('revision3-story-workspace-create-npc'))
            .hitTestable(),
        findsOneWidget,
      );

      final search = find.byKey(const Key('revision3-story-workspace-search'));
      await tester.scrollUntilVisible(
        search,
        60,
        scrollable: find
            .descendant(of: chrome, matching: find.byType(Scrollable))
            .first,
      );
      await tester.pump();
      expect(search.hitTestable(), findsOneWidget);
      expect(
        find
            .byKey(Key('revision3-story-workspace-entity-$_npcId'))
            .hitTestable(),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'checkpoint and project changes close compact sheets and stale actions fail closed',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 600));
      var root = 'root-a';
      var projectId = _projectA;
      var revision = 7;
      var head = 'head-7';
      var index = _fixture();
      var editOverviewCalls = 0;
      var editStoryCalls = 0;
      var editLogicCalls = 0;
      var inspectQuestCalls = 0;
      var inspectNpcCalls = 0;
      var externalEntityCalls = 0;
      var externalAssetCalls = 0;
      late StateSetter rebuild;

      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: _workspace(
                  root: root,
                  projectId: projectId,
                  revision: revision,
                  head: head,
                  load: () async => index,
                  editQuestOutline: (_, _) async => editOverviewCalls++,
                  editQuestContext: (_, _) async => editStoryCalls++,
                  editQuestTransitions: (_, _) async => editLogicCalls++,
                  inspectQuestSource: (_, _) async => inspectQuestCalls++,
                  inspectNpcSource: (_, _) async => inspectNpcCalls++,
                  onOpenExternalEntity: (_) => externalEntityCalls++,
                  onOpenExternalAsset: (_) => externalAssetCalls++,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pumpAndSettle();
      final questWorkbench = tester.widget<Revision3StoryEntityWorkbench>(
        find.byType(Revision3StoryEntityWorkbench),
      );
      final staleQuestActions = questWorkbench.actions;
      expect(staleQuestActions.editOverview, isNotNull);
      expect(staleQuestActions.editStory, isNotNull);
      expect(staleQuestActions.editLogic, isNotNull);
      expect(staleQuestActions.inspectQuest, isNotNull);

      rebuild(() {
        revision = 8;
        head = 'head-8';
        index = _fixture(revision: 8);
      });
      await tester.pump();
      await staleQuestActions.editOverview!();
      await staleQuestActions.editStory!();
      await staleQuestActions.editLogic!();
      await staleQuestActions.inspectQuest!();
      staleQuestActions.openEntity(_moduleId);
      staleQuestActions.openAsset(_artifactSha);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(editOverviewCalls, 0);
      expect(editStoryCalls, 0);
      expect(editLogicCalls, 0);
      expect(inspectQuestCalls, 0);
      expect(externalEntityCalls, 0);
      expect(externalAssetCalls, 0);

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
      );
      await tester.pumpAndSettle();
      final npcWorkbench = tester.widget<Revision3StoryEntityWorkbench>(
        find.byType(Revision3StoryEntityWorkbench),
      );
      final staleNpcInspect = npcWorkbench.actions.inspectNpc!;

      rebuild(() {
        root = 'root-b';
        projectId = _projectB;
        index = _fixture(projectId: _projectB, revision: 8);
      });
      await tester.pump();
      await staleNpcInspect();
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(inspectNpcCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'disposing the project workspace removes its open details sheet',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 600));
      var showWorkspace = true;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: showWorkspace
                    ? _workspace(load: () async => _fixture())
                    : const SizedBox(key: Key('project-workspace-closed')),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );

      rebuild(() => showWorkspace = false);
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('project-workspace-closed')), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(find.byType(Revision3StoryEntityWorkbench), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('rejects stale async results after an exact revision change', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final oldLoad = Completer<Revision3ContentIndex>();
    final freshLoad = Completer<Revision3ContentIndex>();
    var revision = 7;
    Revision3StoryWorkspaceLoader loader = () => oldLoad.future;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                revision: revision,
                head: 'head-$revision',
                load: loader,
              ),
            );
          },
        ),
      ),
    );
    await tester.pump();

    rebuild(() {
      revision = 8;
      loader = () => freshLoad.future;
    });
    await tester.pump();
    oldLoad.complete(_fixture(projectName: 'Stale result'));
    await tester.pump();
    expect(find.text('Stale result'), findsNothing);
    expect(
      find.byKey(const Key('revision3-story-workspace-loading')),
      findsOneWidget,
    );

    freshLoad.complete(_fixture(revision: 8, projectName: 'Fresh result'));
    await tester.pumpAndSettle();
    expect(find.text('2 drafts / revision 8'), findsOneWidget);
    expect(find.text('Stale result'), findsNothing);
  });

  testWidgets('shows an exact load error and retries', (tester) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var calls = 0;
    await _pumpWorkspace(
      tester,
      load: () {
        calls++;
        if (calls == 1) return Future.error(StateError('offline'));
        return Future.value(_fixture());
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-story-workspace-error')),
      findsOneWidget,
    );
    expect(find.textContaining('offline'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-story-workspace-retry')));
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('2 drafts / revision 7'), findsOneWidget);
  });

  testWidgets('rejects a loader result from another checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    await _pumpWorkspace(tester, revision: 8, load: () async => _fixture());
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-story-workspace-error')),
      findsOneWidget,
    );
    expect(
      find.text('Story index does not match this checkpoint.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-story-workspace-list')),
      findsNothing,
    );
  });

  testWidgets('root and exact head changes independently force reloads', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var root = 'root-a';
    var head = 'head-a';
    var calls = 0;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                root: root,
                head: head,
                load: () async {
                  calls++;
                  return _fixture();
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
    expect(calls, 1, reason: 'loader closure identity is not a checkpoint');
    rebuild(() => head = 'head-b');
    await tester.pumpAndSettle();
    expect(calls, 2);
    rebuild(() => root = 'root-b');
    await tester.pumpAndSettle();
    expect(calls, 3);
  });

  testWidgets('controller selects a just-created exact next-revision entity', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3StoryWorkspaceController();
    addTearDown(controller.dispose);
    var revision = 7;
    var index = _fixture(includeQuest: false);
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                revision: revision,
                head: 'head-$revision',
                load: () async => index,
                controller: controller,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-filter-npc')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-story-workspace-search')),
      'guard',
    );
    final selected = controller.selectEntityAtRevision(
      entityId: _questId,
      projectRevision: 8,
      section: Revision3StoryWorkbenchSection.logic,
    );
    bool? resolved;
    selected.then((value) => resolved = value);
    await tester.pump();
    expect(resolved, isNull);

    rebuild(() {
      revision = 8;
      index = _fixture(revision: 8);
    });
    await tester.pumpAndSettle();

    expect(await selected, isTrue);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-story-workspace-search')),
          )
          .controller!
          .text,
      isEmpty,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(const Key('revision3-story-workspace-filter-all')),
          )
          .selected,
      isTrue,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(Key('revision3-story-workbench-tab-logic-$_questId')),
          )
          .selected,
      isTrue,
    );
    expect(
      await controller.selectEntityAtRevision(
        entityId: _moduleId,
        projectRevision: 8,
      ),
      isFalse,
    );
    final unresolved = controller.selectEntityAtRevision(
      entityId: _npcId,
      projectRevision: 9,
    );
    controller.dispose();
    expect(await unresolved, isFalse);
  });

  testWidgets('routes non-Story entities and assets to external owners', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    String? externalEntity;
    String? externalAsset;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      onOpenExternalEntity: (value) => externalEntity = value,
      onOpenExternalAsset: (value) => externalAsset = value,
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
    );
    await tester.pump();
    final references = find.byKey(
      Key('revision3-story-workbench-tab-references-$_questId'),
    );
    await tester.ensureVisible(references);
    await tester.tap(references);
    await tester.pump();

    final moduleReference = find.byKey(
      Key('revision3-story-workbench-outgoing-$_questId-draft_script_module-0'),
    );
    await tester.ensureVisible(moduleReference);
    await tester.tap(moduleReference);
    expect(externalEntity, _moduleId);

    final assetReference = find.byKey(
      Key(
        'revision3-story-workbench-outgoing-asset-$_questId-quest_collision_artifact-0',
      ),
    );
    await tester.ensureVisible(assetReference);
    await tester.tap(assetReference);
    expect(externalAsset, _artifactSha);
  });
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  required Revision3StoryWorkspaceLoader load,
  int revision = 7,
  Revision3StoryWorkspaceController? controller,
  Revision3StoryWorkspaceCreateAction? createNpcDraft,
  Revision3StoryWorkspaceCreateAction? createQuestDraft,
  ValueChanged<String>? onOpenExternalEntity,
  ValueChanged<String>? onOpenExternalAsset,
  Revision3StoryWorkspaceEntityAction? editQuestOutline,
  Revision3StoryWorkspaceEntityAction? editQuestContext,
  Revision3StoryWorkspaceEntityAction? editQuestTransitions,
  Revision3StoryWorkspaceEntityAction? inspectQuestSource,
  Revision3StoryWorkspaceEntityAction? inspectNpcSource,
  Revision3StoryWorkspaceCopy? copy,
  String? createNpcDraftDisabledReason,
  String? createQuestDraftDisabledReason,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: _workspace(
        revision: revision,
        head: 'head-$revision',
        load: load,
        controller: controller,
        createNpcDraft: createNpcDraft,
        createQuestDraft: createQuestDraft,
        onOpenExternalEntity: onOpenExternalEntity,
        onOpenExternalAsset: onOpenExternalAsset,
        editQuestOutline: editQuestOutline,
        editQuestContext: editQuestContext,
        editQuestTransitions: editQuestTransitions,
        inspectQuestSource: inspectQuestSource,
        inspectNpcSource: inspectNpcSource,
        copy: copy,
        createNpcDraftDisabledReason: createNpcDraftDisabledReason,
        createQuestDraftDisabledReason: createQuestDraftDisabledReason,
      ),
    ),
  ),
);

Revision3StoryWorkspace _workspace({
  String root = 'root-a',
  String projectId = _projectA,
  int revision = 7,
  String head = 'head-7',
  required Revision3StoryWorkspaceLoader load,
  Revision3StoryWorkspaceController? controller,
  Revision3StoryWorkspaceCreateAction? createNpcDraft,
  Revision3StoryWorkspaceCreateAction? createQuestDraft,
  ValueChanged<String>? onOpenExternalEntity,
  ValueChanged<String>? onOpenExternalAsset,
  Revision3StoryWorkspaceEntityAction? editQuestOutline,
  Revision3StoryWorkspaceEntityAction? editQuestContext,
  Revision3StoryWorkspaceEntityAction? editQuestTransitions,
  Revision3StoryWorkspaceEntityAction? inspectQuestSource,
  Revision3StoryWorkspaceEntityAction? inspectNpcSource,
  Revision3StoryWorkspaceCopy? copy,
  String? createNpcDraftDisabledReason,
  String? createQuestDraftDisabledReason,
}) => Revision3StoryWorkspace(
  projectRoot: root,
  projectId: projectId,
  projectRevision: revision,
  projectHeadCanonicalJson: head,
  load: load,
  copy: copy ?? _copy,
  controller: controller,
  createNpcDraft: createNpcDraft,
  createQuestDraft: createQuestDraft,
  createNpcDraftDisabledReason: createNpcDraft == null
      ? createNpcDraftDisabledReason ?? 'NPC creation is not configured.'
      : null,
  createQuestDraftDisabledReason: createQuestDraft == null
      ? createQuestDraftDisabledReason ?? 'Quest creation is not configured.'
      : null,
  onOpenExternalEntity: onOpenExternalEntity ?? (_) {},
  onOpenExternalAsset: onOpenExternalAsset ?? (_) {},
  editQuestOutline: editQuestOutline,
  editQuestContext: editQuestContext,
  editQuestTransitions: editQuestTransitions,
  inspectQuestSource: inspectQuestSource,
  inspectNpcSource: inspectNpcSource,
);

Revision3ContentIndex _fixture({
  String projectId = _projectA,
  int revision = 7,
  String projectName = 'Fixture project',
  bool includeNpc = true,
  bool includeQuest = true,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': projectName,
  'project_version': '0.1.0',
  'project_author': 'GORE',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 123, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{
    if (includeNpc) 'npc_draft': 1,
    if (includeQuest) 'quest_draft': 1,
    if (includeQuest) 'script_module': 1,
  },
  'entities': <Object?>[
    if (includeNpc)
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
    if (includeQuest)
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
              'project_id': projectId,
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
    if (includeQuest)
      <String, Object?>{
        'id': _moduleId,
        'kind': 'script_module',
        'display_name': 'Find Homer source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.quest-draft',
          'generator_version': 2,
          'owner': <String, Object?>{
            'project_id': projectId,
            'entity_id': _questId,
            'expected_kind': 'quest_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.quest-draft',
            'generator_version': 2,
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
              'project_id': projectId,
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
    if (includeQuest)
      <String, Object?>{
        'sha256': _artifactSha,
        'byte_len': 8192,
        'media_type':
            'application/vnd.gore.quest-collision-capability+json;version=2',
        'class': 'quest_collision_artifact',
      },
  ],
});
