import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_content_library.dart';
import 'package:gore_mod/project/revision3_story_entity_workbench.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _questId = '33333333333333333333333333333333';
const _moduleId = '44444444444444444444444444444444';
const _missingId = '55555555555555555555555555555555';
const _npcModuleId = '2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f';
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
    expect(find.text('4 entities / 2 assets / revision 7'), findsOneWidget);
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
    expect(find.text('Gate Guard'), findsWidgets);
    expect(
      find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
      findsOneWidget,
    );
    final profile = find.byKey(
      Key('revision3-story-workbench-section-profile-$_npcId'),
    );
    final plannedCapabilities = find.byKey(
      const Key('revision3-story-workbench-npc-planned-capabilities'),
    );
    await tester.scrollUntilVisible(
      plannedCapabilities,
      160,
      scrollable: find.descendant(
        of: profile,
        matching: find.byType(Scrollable),
      ),
    );
    await tester.pumpAndSettle();

    expect(plannedCapabilities, findsOneWidget);
    expect(find.text('Story, Routine, Inventory'), findsOneWidget);
    expect(
      find.text(
        'Quest and story relationships are not modeled for NPC drafts yet.',
      ),
      findsNothing,
      reason: 'planned NPC domains start collapsed',
    );
  });

  testWidgets(
    'keeps Quest overview action visible but disabled without lease',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      await _pumpLoadedLibrary(tester);

      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pump();

      final action = find.byKey(
        Key('revision3-story-workbench-action-edit-overview-$_questId'),
      );
      expect(action, findsOneWidget);
      expect(
        tester
            .widget<ListTile>(
              find.descendant(of: action, matching: find.byType(ListTile)),
            )
            .enabled,
        isFalse,
      );
      expect(
        find.descendant(of: action, matching: find.text('Not modeled yet')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'wide Story discovery forwards the exact Quest without duplicate tools',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1600, 900));
      final pending = Completer<void>();
      var calls = 0;
      var editorCalls = 0;
      var inspectionCalls = 0;
      Revision3ContentIndex? openedIndex;
      Revision3ContentEntity? openedEntity;
      await _pumpLoadedLibrary(
        tester,
        openStoryDraftLabel: 'Continue Quest in Story',
        openStoryDraftDescription:
            'Use the canonical Story workspace for the complete Quest.',
        openStoryDraftInStory: (index, entity) {
          calls++;
          openedIndex = index;
          openedEntity = entity;
          return pending.future;
        },
        editQuestOutline: (index, entity) async => editorCalls++,
        editQuestContext: (index, entity) async => editorCalls++,
        editQuestTransitions: (index, entity) async => editorCalls++,
        inspectQuestSource: (index, entity) async => inspectionCalls++,
      );

      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pump();

      final continuation = find.byKey(
        const Key('revision3-content-open-story-continuation'),
      );
      final button = find.byKey(Key('revision3-content-open-story-$_questId'));
      expect(continuation, findsOneWidget);
      expect(
        find.byKey(
          const ValueKey('revision3-content-story-discovery-$_questId'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-content-open-story-wide')),
        findsOneWidget,
      );
      expect(find.text('Continue Quest in Story'), findsOneWidget);
      expect(
        find.text('Use the canonical Story workspace for the complete Quest.'),
        findsOneWidget,
      );
      expect(find.text('Quest draft'), findsOneWidget);
      expect(find.textContaining('Ask Asghan about Homer'), findsOneWidget);
      expect(find.text('No unresolved project references'), findsOneWidget);
      expect(find.text('Semantic identity'), findsNothing);
      expect(find.text('Origin'), findsNothing);
      expect(find.text('Entity revision'), findsNothing);
      expect(find.text('Stable ID'), findsNothing);
      expect(
        find.byKey(Key('revision3-story-workbench-$_projectId-$_questId')),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-action-edit-overview-$_questId'),
        ),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-action-inspect-quest_draft-$_questId'),
        ),
        findsNothing,
      );

      final firstPress = tester.widget<FilledButton>(button).onPressed;
      expect(firstPress, isNotNull);
      await tester.tap(button);
      await tester.pump();

      expect(calls, 1);
      expect(
        identical(openedIndex?.entityById(_questId), openedEntity),
        isTrue,
      );
      expect(openedEntity?.id, _questId);
      expect(tester.widget<FilledButton>(button).onPressed, isNull);
      firstPress!();
      await tester.pump();
      expect(calls, 1, reason: 'one exact handoff remains single-flight');
      expect(editorCalls, 0);
      expect(inspectionCalls, 0);

      pending.complete();
      await tester.pumpAndSettle();
      expect(tester.widget<FilledButton>(button).onPressed, isNotNull);
    },
  );

  testWidgets(
    'compact NPC continuation closes its details sheet before handoff',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      var calls = 0;
      var editorCalls = 0;
      var inspectionCalls = 0;
      var sheetWasVisible = true;
      String? openedId;
      await _pumpLoadedLibrary(
        tester,
        openStoryDraftInStory: (index, entity) async {
          calls++;
          openedId = entity.id;
          expect(identical(index.entityById(entity.id), entity), isTrue);
          sheetWasVisible = find.byType(BottomSheet).evaluate().isNotEmpty;
        },
        editNpcProfile: (index, entity) async => editorCalls++,
        inspectNpcSource: (index, entity) async => inspectionCalls++,
      );

      await tester.tap(find.byKey(Key('revision3-content-entity-$_npcId')));
      await tester.pumpAndSettle();

      expect(find.byType(BottomSheet), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-content-open-story-compact')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workbench-$_projectId-$_npcId')),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
        ),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-action-inspect-npc_draft-$_npcId'),
        ),
        findsNothing,
      );
      final button = find.byKey(Key('revision3-content-open-story-$_npcId'));
      await tester.ensureVisible(button);
      await tester.tap(button);
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(openedId, _npcId);
      expect(sheetWasVisible, isFalse);
      expect(editorCalls, 0);
      expect(inspectionCalls, 0);
      expect(find.byType(BottomSheet), findsNothing);
    },
  );

  testWidgets('primary Story continuation is keyboard reachable', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1600, 900));
    var calls = 0;
    await _pumpLoadedLibrary(
      tester,
      openStoryDraftInStory: (index, entity) async => calls++,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();

    final buttonKey = Key('revision3-content-open-story-$_questId');
    bool buttonHasPrimaryFocus() {
      final context = tester.binding.focusManager.primaryFocus?.context;
      if (context == null) return false;
      if (context.widget.key == buttonKey) return true;
      var found = false;
      context.visitAncestorElements((ancestor) {
        found = ancestor.widget.key == buttonKey;
        return !found;
      });
      return found;
    }

    for (var step = 0; step < 40 && !buttonHasPrimaryFocus(); step++) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
    }
    expect(buttonHasPrimaryFocus(), isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(calls, 1);
  });

  testWidgets('canonical discovery refuses a false-success Problems deep link', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(
      tester,
      controller: controller,
      openStoryDraftInStory: (index, entity) async {},
    );

    final opened = await controller.openEntityProblemsById(_questId);
    await tester.pumpAndSettle();

    expect(opened, isFalse);
    expect(find.byType(BottomSheet), findsNothing);
    expect(
      find.byKey(
        Key(
          'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
        ),
      ),
      findsNothing,
    );
  });

  testWidgets(
    'disabled canonical handoff stays explicit and usable in a long compact layout',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      const reason =
          'Öffne das Projekt erneut, bevor du diesen Quest-Entwurf im kanonischen Story-Arbeitsbereich weiterbearbeitest.';
      final controller = Revision3ContentLibraryController();
      var editorCalls = 0;
      await _pumpLoadedLibrary(
        tester,
        controller: controller,
        openStoryDraftInStoryDisabledReason: reason,
        editQuestOutline: (index, entity) async => editorCalls++,
        inspectQuestSource: (index, entity) async => editorCalls++,
      );

      expect(await controller.openEntityProblemsById(_questId), isFalse);
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pumpAndSettle();

      final button = find.byKey(Key('revision3-content-open-story-$_questId'));
      expect(find.byType(BottomSheet), findsOneWidget);
      expect(
        find.byKey(
          const ValueKey('revision3-content-story-discovery-$_questId'),
        ),
        findsOneWidget,
      );
      expect(find.byTooltip(reason), findsOneWidget);
      expect(find.text(reason), findsOneWidget);
      expect(tester.widget<FilledButton>(button).onPressed, isNull);
      expect(
        find.byKey(Key('revision3-story-workbench-$_projectId-$_questId')),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-action-edit-overview-$_questId'),
        ),
        findsNothing,
      );
      expect(editorCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'enabled handoff becomes stale when canonical Story is disabled',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1600, 900));
      const reason = 'Reopen the project before continuing in Story.';
      var disabled = false;
      var calls = 0;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: Revision3ContentLibrary(
                  projectRoot: 'managed-root',
                  projectId: _projectId,
                  projectRevision: 7,
                  projectHeadCanonicalJson: 'canonical-head-7',
                  load: () async => _fixture(),
                  openStoryDraftInStory: disabled
                      ? null
                      : (index, entity) async => calls++,
                  openStoryDraftInStoryDisabledReason: disabled ? reason : null,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pump();
      final button = find.byKey(Key('revision3-content-open-story-$_questId'));
      final stalePress = tester.widget<FilledButton>(button).onPressed;
      expect(stalePress, isNotNull);

      rebuild(() => disabled = true);
      await tester.pump();
      expect(tester.widget<FilledButton>(button).onPressed, isNull);
      expect(find.text(reason), findsOneWidget);
      expect(
        find.byKey(Key('revision3-story-workbench-$_projectId-$_questId')),
        findsNothing,
      );

      stalePress!();
      await tester.pumpAndSettle();
      expect(calls, 0);
    },
  );

  testWidgets(
    'stale compact continuation is inert after exact project reload',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      var revision = 7;
      var calls = 0;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: Revision3ContentLibrary(
                  projectRoot: 'managed-root',
                  projectId: _projectId,
                  projectRevision: revision,
                  projectHeadCanonicalJson: 'canonical-head-$revision',
                  load: () async => _fixture(revision: revision),
                  openStoryDraftInStory: (index, entity) async => calls++,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pumpAndSettle();
      final stalePress = tester
          .widget<FilledButton>(
            find.byKey(Key('revision3-content-open-story-$_questId')),
          )
          .onPressed;
      expect(stalePress, isNotNull);

      rebuild(() => revision = 8);
      await tester.pumpAndSettle();
      expect(find.byType(BottomSheet), findsOneWidget);

      stalePress!();
      await tester.pumpAndSettle();

      expect(calls, 0);
      expect(find.byType(BottomSheet), findsNothing);
      expect(find.text('4 entities / 2 assets / revision 8'), findsOneWidget);
    },
  );

  testWidgets('failed Story handoff is friendly and hides raw details', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1600, 900));
    await _pumpLoadedLibrary(
      tester,
      openStoryDraftInStory: (index, entity) async {
        throw StateError(r'C:\private\story.json $_questId');
      },
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await tester.tap(find.byKey(Key('revision3-content-open-story-$_questId')));
    await tester.pumpAndSettle();

    expect(
      find.text('Story could not be opened. The project was not changed.'),
      findsOneWidget,
    );
    expect(find.textContaining('private'), findsNothing);
    expect(find.textContaining(r'C:\'), findsNothing);
  });

  testWidgets('failed compact Story handoff stays visibly actionable', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    await _pumpLoadedLibrary(
      tester,
      openStoryDraftFailureMessage:
          'Story konnte nicht geöffnet werden. Das Projekt blieb unverändert.',
      openStoryDraftInStory: (index, entity) async {
        throw StateError(r'C:\private\compact-story.json $_questId');
      },
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pumpAndSettle();
    final button = find.byKey(Key('revision3-content-open-story-$_questId'));
    await tester.ensureVisible(button);
    await tester.tap(button);
    await tester.pumpAndSettle();

    expect(find.byType(BottomSheet), findsNothing);
    expect(find.byType(SnackBar), findsOneWidget);
    expect(
      find.text(
        'Story konnte nicht geöffnet werden. Das Projekt blieb unverändert.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('private'), findsNothing);
    expect(find.textContaining(r'C:\'), findsNothing);
  });

  testWidgets('no Story callback renders no misleading continuation', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1600, 900));
    await _pumpLoadedLibrary(tester);

    expect(
      find.byKey(const Key('revision3-content-open-story-continuation')),
      findsNothing,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-content-open-story-continuation')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-content-open-story-$_questId')),
      findsNothing,
    );
  });

  testWidgets(
    'canonical Story handoff leaves non-Story content inspection intact',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1600, 900));
      await _pumpLoadedLibrary(
        tester,
        openStoryDraftInStory: (index, entity) async {},
      );

      await tester.tap(find.byKey(Key('revision3-content-entity-$_moduleId')));
      await tester.pump();

      expect(
        find.byKey(
          const ValueKey('revision3-content-entity-details-$_moduleId'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-content-entity-details')),
        findsOneWidget,
      );
      expect(find.text('Semantic identity'), findsOneWidget);
      expect(find.text('Stable ID'), findsOneWidget);
      expect(find.text(_moduleId), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-content-open-story-continuation')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'fallback keeps bounded Quest editors in one Overview without duplicate tabs',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var outlineCalls = 0;
      var contextCalls = 0;
      var transitionCalls = 0;
      await _pumpLoadedLibrary(
        tester,
        editQuestOutline: (index, quest) async => outlineCalls++,
        editQuestContext: (index, quest) async => contextCalls++,
        editQuestTransitions: (index, quest) async => transitionCalls++,
      );
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pump();

      final overviewAction = find.byKey(
        Key('revision3-story-workbench-action-edit-overview-$_questId'),
      );
      expect(
        tester
            .widget<ListTile>(
              find.descendant(
                of: overviewAction,
                matching: find.byType(ListTile),
              ),
            )
            .enabled,
        isTrue,
      );
      await tester.tap(overviewAction);
      await tester.pumpAndSettle();
      expect(outlineCalls, 1);
      expect(
        find.byKey(Key('revision3-story-workbench-$_projectId-$_questId')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-tab-story-$_questId'),
          skipOffstage: false,
        ),
        findsNothing,
      );
      expect(
        find.byKey(
          Key('revision3-story-workbench-tab-logic-$_questId'),
          skipOffstage: false,
        ),
        findsNothing,
      );
      final storyAction = find.byKey(
        Key('revision3-story-workbench-action-edit-story-$_questId'),
      );
      expect(storyAction, findsOneWidget);
      await tester.tap(storyAction);
      await tester.pumpAndSettle();
      expect(contextCalls, 1);

      final overviewSection = find.byKey(
        Key(
          'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.overview.name}-$_questId',
        ),
      );
      final overviewScroll = find.descendant(
        of: overviewSection,
        matching: find.byType(Scrollable),
      );
      final logicAction = find.byKey(
        Key('revision3-story-workbench-action-edit-logic-$_questId'),
        skipOffstage: false,
      );
      await tester.scrollUntilVisible(
        logicAction,
        80,
        scrollable: overviewScroll,
      );
      await tester.ensureVisible(logicAction);
      await tester.pumpAndSettle();
      expect(logicAction.hitTestable(), findsOneWidget);
      await tester.tap(logicAction.hitTestable());
      await tester.pumpAndSettle();
      expect(transitionCalls, 1);
    },
  );

  testWidgets('Source & checks routes the selected exact Quest separately', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var inspectionCalls = 0;
    String? inspectedQuest;
    await _pumpLoadedLibrary(
      tester,
      inspectQuestSource: (index, quest) async {
        inspectionCalls++;
        expect(index.projectId, _projectId);
        inspectedQuest = quest.id;
      },
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.problemsChecks,
      _questId,
    );
    final action = find.byKey(
      Key('revision3-story-workbench-action-inspect-quest_draft-$_questId'),
    );
    expect(action, findsOneWidget);
    expect(
      tester
          .widget<ListTile>(
            find.descendant(of: action, matching: find.byType(ListTile)),
          )
          .enabled,
      isTrue,
    );
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(inspectionCalls, 1);
    expect(inspectedQuest, _questId);
  });

  testWidgets('disabled Quest inspection shows its supplied setup reason', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(
      tester,
      inspectQuestSourceDisabledReason:
          'Select a game installation before running compiler checks.',
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.problemsChecks,
      _questId,
    );

    final action = find.byKey(
      Key('revision3-story-workbench-action-inspect-quest_draft-$_questId'),
    );
    expect(
      tester
          .widget<ListTile>(
            find.descendant(of: action, matching: find.byType(ListTile)),
          )
          .enabled,
      isFalse,
    );
    expect(
      find.text('Select a game installation before running compiler checks.'),
      findsOneWidget,
    );
  });

  testWidgets('NPC checks route the exact NPC without a game root', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var inspectionCalls = 0;
    String? inspectedNpc;
    await _pumpLoadedLibrary(
      tester,
      inspectNpcSource: (index, npc) async {
        inspectionCalls++;
        expect(index.projectId, _projectId);
        inspectedNpc = npc.id;
      },
    );

    final checksTab = find.byKey(
      Key('revision3-story-workbench-tab-problemsChecks-$_npcId'),
    );
    await tester.ensureVisible(checksTab);
    await tester.tap(checksTab);
    await tester.pumpAndSettle();
    final action = find.byKey(
      Key('revision3-story-workbench-action-inspect-npc_draft-$_npcId'),
    );
    expect(action, findsOneWidget);
    expect(
      tester
          .widget<ListTile>(
            find.descendant(of: action, matching: find.byType(ListTile)),
          )
          .enabled,
      isTrue,
    );
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(inspectionCalls, 1);
    expect(inspectedNpc, _npcId);
  });

  testWidgets('NPC profile edit routes the exact NPC from Profile', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var editCalls = 0;
    String? editedNpc;
    await _pumpLoadedLibrary(
      tester,
      editNpcProfile: (index, npc) async {
        editCalls++;
        expect(index.projectId, _projectId);
        editedNpc = npc.id;
      },
    );

    final action = find.byKey(
      Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
    );
    expect(action, findsOneWidget);
    expect(
      tester
          .widget<ListTile>(
            find.descendant(of: action, matching: find.byType(ListTile)),
          )
          .enabled,
      isTrue,
    );
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(editCalls, 1);
    expect(editedNpc, _npcId);
  });

  testWidgets('current Quest keeps stable-slot outline in bounded Overview', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var outlineCalls = 0;
    var contextCalls = 0;
    await _pumpLoadedLibrary(
      tester,
      editQuestOutline: (index, quest) async => outlineCalls++,
      editQuestContext: (index, quest) async => contextCalls++,
      editQuestTransitions: (index, quest) async {},
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    final overviewAction = find.byKey(
      Key('revision3-story-workbench-action-edit-overview-$_questId'),
    );
    expect(
      tester
          .widget<ListTile>(
            find.descendant(
              of: overviewAction,
              matching: find.byType(ListTile),
            ),
          )
          .enabled,
      isTrue,
    );
    await tester.tap(overviewAction);
    await tester.pumpAndSettle();
    expect(outlineCalls, 1);
    final storyAction = find.byKey(
      Key('revision3-story-workbench-action-edit-story-$_questId'),
    );
    expect(storyAction, findsOneWidget);
    await tester.tap(storyAction);
    await tester.pumpAndSettle();
    expect(contextCalls, 1);
  });

  test('arbitrary or obsolete Quest generators are rejected', () {
    expect(
      () => _fixture(
        questGeneratorVersion: 4,
        questGeneratorId: 'gore-authoring.unrelated-generator',
      ),
      throwsFormatException,
    );
    expect(() => _fixture(questGeneratorVersion: 2), throwsFormatException);
  });

  testWidgets('routes Quest context editing from Overview', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var contextCalls = 0;
    await _pumpLoadedLibrary(
      tester,
      editQuestOutline: (index, quest) async {},
      editQuestContext: (index, quest) async => contextCalls++,
    );
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    final storyAction = find.byKey(
      Key('revision3-story-workbench-action-edit-story-$_questId'),
    );
    expect(storyAction, findsOneWidget);
    await tester.tap(storyAction);
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
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.references,
      _questId,
    );
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

    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.references,
      _questId,
    );
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
    expect(
      find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
      findsOneWidget,
    );
    final planned = find.byKey(
      const Key('revision3-story-workbench-npc-planned-capabilities'),
    );
    await tester.scrollUntilVisible(
      planned,
      100,
      scrollable: find.descendant(
        of: find.byKey(
          Key(
            'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.profile.name}-$_npcId',
          ),
        ),
        matching: find.byType(Scrollable),
      ),
    );
    await tester.pumpAndSettle();
    expect(planned, findsOneWidget);
    expect(find.text('Story, Routine, Inventory'), findsOneWidget);
    expect(
      find.text('Routine and world placement are not modeled yet.'),
      findsNothing,
      reason: 'planned NPC domains remain collapsed on compact entry',
    );
  });

  for (final viewport in <({String label, Size size})>[
    (label: '1280x720', size: Size(1280, 720)),
    (label: '1600x900', size: Size(1600, 900)),
  ]) {
    testWidgets(
      'Home ${viewport.label} uses the details sheet for a fully interactive action',
      (tester) async {
        await _setSurfaceSize(tester, viewport.size);
        await _pumpHomeContentViewportLibrary(tester);

        expect(find.byType(BottomSheet), findsNothing);
        await _selectQuestFromViewportList(tester);

        expect(find.byType(BottomSheet), findsOneWidget);
        await _expectOverviewActionFullyInteractive(tester);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'Home 1920x1080 keeps a fully visible action in the wide details pane',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1920, 1080));
      await _pumpHomeContentViewportLibrary(tester);

      await _selectQuestFromViewportList(tester);

      expect(find.byType(BottomSheet), findsNothing);
      expect(
        find.byKey(const Key('revision3-content-entity-details')),
        findsOneWidget,
      );
      await _expectOverviewActionFullyInteractive(tester);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('compact Quest fallback closes before its bounded editor', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    var outlineCalls = 0;
    var sheetWasVisibleAtCallback = false;
    await _pumpLoadedLibrary(
      tester,
      editQuestOutline: (index, quest) async {
        outlineCalls++;
        sheetWasVisibleAtCallback = find
            .byKey(const Key('revision3-content-entity-details'))
            .evaluate()
            .isNotEmpty;
      },
    );

    Future<void> openOutline() async {
      await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-content-entity-details')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(
          Key('revision3-story-workbench-action-edit-overview-$_questId'),
        ),
      );
      await tester.pumpAndSettle();
    }

    await openOutline();
    expect(outlineCalls, 1);
    expect(sheetWasVisibleAtCallback, isFalse);
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );

    await openOutline();
    expect(outlineCalls, 2);
    expect(sheetWasVisibleAtCallback, isFalse);
  });

  testWidgets('compact NPC sheet closes before routing checks', (tester) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    var inspectionCalls = 0;
    var sheetWasVisibleAtCallback = false;
    String? inspectedNpc;
    await _pumpLoadedLibrary(
      tester,
      inspectNpcSource: (index, npc) async {
        inspectionCalls++;
        inspectedNpc = npc.id;
        sheetWasVisibleAtCallback = find
            .byKey(const Key('revision3-content-entity-details'))
            .evaluate()
            .isNotEmpty;
      },
    );

    await tester.tap(find.byKey(Key('revision3-content-entity-$_npcId')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsOneWidget,
    );
    final checksTab = find.byKey(
      Key('revision3-story-workbench-tab-problemsChecks-$_npcId'),
    );
    await tester.ensureVisible(checksTab);
    await tester.tap(checksTab);
    await tester.pumpAndSettle();
    final action = find.byKey(
      Key('revision3-story-workbench-action-inspect-npc_draft-$_npcId'),
    );
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(inspectionCalls, 1);
    expect(inspectedNpc, _npcId);
    expect(sheetWasVisibleAtCallback, isFalse);
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );
  });

  testWidgets('compact NPC sheet closes before profile editing', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    var editCalls = 0;
    var sheetWasVisibleAtCallback = false;
    await _pumpLoadedLibrary(
      tester,
      editNpcProfile: (index, npc) async {
        editCalls++;
        expect(index.projectId, _projectId);
        expect(npc.id, _npcId);
        sheetWasVisibleAtCallback = find
            .byKey(const Key('revision3-content-entity-details'))
            .evaluate()
            .isNotEmpty;
      },
    );

    await tester.tap(find.byKey(Key('revision3-content-entity-$_npcId')));
    await tester.pumpAndSettle();
    final action = find.byKey(
      Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
    );
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(editCalls, 1);
    expect(sheetWasVisibleAtCallback, isFalse);
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );
  });

  testWidgets('shows honest draft boundaries and disabled unmodeled sections', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    expect(find.text('Draft only'), findsOneWidget);
    expect(find.text('Build blocked'), findsOneWidget);
    expect(find.text('Runtime not verified'), findsOneWidget);

    final profile = find.byKey(
      Key(
        'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.profile.name}-$_npcId',
      ),
    );
    final plannedTitle = find.text('Story, Routine, Inventory');
    await tester.scrollUntilVisible(
      plannedTitle,
      100,
      scrollable: find.descendant(
        of: profile,
        matching: find.byType(Scrollable),
      ),
    );
    await tester.pumpAndSettle();
    expect(plannedTitle, findsOneWidget);
    expect(
      find.text('Routine and world placement are not modeled yet.'),
      findsNothing,
      reason: 'the honest planned-work summary starts collapsed',
    );
    final semantics = tester.ensureSemantics();
    final plannedSemantics = find.semantics.byLabel(
      RegExp('Story, Routine, Inventory'),
    );
    expect(plannedSemantics, findsOneWidget);
    tester.semantics.tap(plannedSemantics);
    await tester.pumpAndSettle();
    expect(
      find.text(
        'Quest and story relationships are not modeled for NPC drafts yet.',
      ),
      findsOneWidget,
    );
    expect(
      find.text('Routine and world placement are not modeled yet.'),
      findsOneWidget,
    );
    expect(
      find.text('Inventory, equipment, and trading are not modeled yet.'),
      findsOneWidget,
    );
    semantics.dispose();

    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.dialogVoice,
      _questId,
    );
    expect(
      find.text(
        'Dialog, localization, and voice relationships are not modeled for Quest drafts yet.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('preserves a section across revision reload and clears deletion', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var revision = 7;
    var includeQuest = true;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3ContentLibrary(
                projectRoot: 'managed-root',
                projectId: _projectId,
                projectRevision: revision,
                projectHeadCanonicalJson: 'head-$revision',
                load: () async =>
                    _fixture(revision: revision, includeQuest: includeQuest),
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.references,
      _questId,
    );

    rebuild(() => revision = 8);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(
              Key(
                'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.references.name}-$_questId',
              ),
            ),
          )
          .selected,
      isTrue,
    );

    rebuild(() {
      revision = 9;
      includeQuest = false;
    });
    await tester.pumpAndSettle();
    expect(find.byKey(Key('revision3-content-entity-$_questId')), findsNothing);

    rebuild(() {
      revision = 10;
      includeQuest = true;
    });
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(
              Key(
                'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.overview.name}-$_questId',
              ),
            ),
          )
          .selected,
      isTrue,
    );
  });

  testWidgets('project switch clears remembered story section', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var root = 'managed-root-a';
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
                projectRevision: 7,
                projectHeadCanonicalJson: 'head',
                load: () async => _fixture(),
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    await _openWorkbenchSection(
      tester,
      Revision3StoryWorkbenchSection.references,
      _questId,
    );

    rebuild(() => root = 'managed-root-b');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(Key('revision3-content-entity-$_questId')));
    await tester.pump();
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(
              Key(
                'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.overview.name}-$_questId',
              ),
            ),
          )
          .selected,
      isTrue,
    );
  });

  testWidgets('controller opens exact IDs and resets local discovery state', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(tester, controller: controller);

    await tester.tap(
      find.byKey(const Key('revision3-content-filter-quest_draft')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'Find Homer',
    );
    await tester.pump();
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsNothing);

    expect(await controller.openEntityById(_npcId), isTrue);
    await tester.pump();
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsOneWidget);
    expect(
      find.byKey(ValueKey('revision3-content-entity-details-$_npcId')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('revision3-content-search')))
          .controller
          ?.text,
      isEmpty,
    );

    expect(await controller.openAssetBySha256(_artifactSha), isTrue);
    await tester.pump();
    expect(
      find.byKey(ValueKey('revision3-content-asset-details-$_artifactSha')),
      findsOneWidget,
    );
    expect(
      await controller.openAssetBySha256(_artifactSha.toUpperCase()),
      isFalse,
      reason: 'SHA navigation is exact rather than normalized or fuzzy',
    );
    expect(await controller.openEntityById('missing'), isFalse);
  });

  testWidgets('checkpoint entity navigation requires exact revision and head', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(tester, controller: controller);

    expect(
      await controller.openEntityProblemsByIdAtCheckpoint(
        _questId,
        projectRevision: 8,
        projectHeadCanonicalJson: 'canonical-head-7',
      ),
      isFalse,
    );
    expect(
      await controller.openEntityProblemsByIdAtCheckpoint(
        _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'another-head',
      ),
      isFalse,
    );
    expect(
      await controller.openEntityProblemsByIdAtCheckpoint(
        _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'canonical-head-7',
      ),
      isTrue,
    );
    await tester.pumpAndSettle();

    final problemsTab = find.byKey(
      Key(
        'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
      ),
    );
    expect(problemsTab, findsOneWidget);
    expect(tester.widget<ChoiceChip>(problemsTab).selected, isTrue);

    expect(
      await controller.openEntityProblemsByIdAtCheckpoint(
        _moduleId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'canonical-head-7',
      ),
      isFalse,
      reason: 'non-Story inspection cannot claim a Story Problems section',
    );
    expect(
      await controller.openEntityByIdAtCheckpoint(
        _moduleId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'another-head',
      ),
      isFalse,
    );
    expect(
      await controller.openEntityByIdAtCheckpoint(
        _moduleId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'canonical-head-7',
      ),
      isTrue,
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('revision3-content-entity-details-$_moduleId')),
      findsOneWidget,
    );
  });

  testWidgets('pre-mount checkpoint request rejects a different head', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController(
      projectIdentity: const Revision3ContentProjectIdentity(
        projectRoot: 'managed-root',
        projectId: _projectId,
      ),
    );
    final opening = controller.openEntityProblemsByIdAtCheckpoint(
      _questId,
      projectRevision: 7,
      projectHeadCanonicalJson: 'stale-head',
    );

    await _pumpLoadedLibrary(tester, controller: controller);

    expect(await opening, isFalse);
    expect(
      find.byKey(
        Key(
          'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
        ),
      ),
      findsNothing,
    );
  });

  testWidgets(
    'pending checkpoint Problems cancels on same-revision head drift and B succeeds',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final controller = Revision3ContentLibraryController();
      final oldHeadReload = Completer<Revision3ContentIndex>();
      final newHeadReload = Completer<Revision3ContentIndex>();
      var calls = 0;
      var head = 'head-a';
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: Revision3ContentLibrary(
                  projectRoot: 'managed-root',
                  projectId: _projectId,
                  projectRevision: 7,
                  projectHeadCanonicalJson: head,
                  controller: controller,
                  load: () {
                    calls++;
                    return switch (calls) {
                      1 => Future.value(_fixture()),
                      2 => oldHeadReload.future,
                      _ => newHeadReload.future,
                    };
                  },
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-content-refresh')));
      await tester.pump();
      bool? oldResolved;
      final oldOpening = controller.openEntityProblemsByIdAtCheckpoint(
        _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-a',
      );
      unawaited(oldOpening.then((value) => oldResolved = value));
      await tester.pump();
      expect(oldResolved, isNull);

      rebuild(() => head = 'head-b');
      await tester.pump();

      expect(await oldOpening, isFalse);
      final newOpening = controller.openEntityProblemsByIdAtCheckpoint(
        _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-b',
      );
      bool? newResolved;
      unawaited(newOpening.then((value) => newResolved = value));
      await tester.pump();
      expect(newResolved, isNull);

      oldHeadReload.complete(_fixture());
      await tester.pump();
      expect(newResolved, isNull);
      newHeadReload.complete(_fixture());
      await tester.pumpAndSettle();

      expect(await newOpening, isTrue);
      final problemsTab = find.byKey(
        Key(
          'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
        ),
      );
      expect(problemsTab, findsOneWidget);
      expect(tester.widget<ChoiceChip>(problemsTab).selected, isTrue);
    },
  );

  testWidgets(
    'buffered compact asset checkpoint cancels on head drift before its sheet',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      final controller = Revision3ContentLibraryController(
        projectIdentity: const Revision3ContentProjectIdentity(
          projectRoot: 'managed-root',
          projectId: _projectId,
        ),
      );
      final headAOpen = Completer<Revision3ContentIndex>();
      final headBOpen = Completer<Revision3ContentIndex>();
      var head = 'head-a';
      var calls = 0;
      late StateSetter rebuild;
      final oldOpening = controller.openAssetBySha256AtCheckpoint(
        _artifactSha,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-a',
      );
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: Revision3ContentLibrary(
                  projectRoot: 'managed-root',
                  projectId: _projectId,
                  projectRevision: 7,
                  projectHeadCanonicalJson: head,
                  controller: controller,
                  load: () {
                    calls++;
                    return calls == 1 ? headAOpen.future : headBOpen.future;
                  },
                ),
              );
            },
          ),
        ),
      );
      await tester.pump();
      expect(find.byType(BottomSheet), findsNothing);

      rebuild(() => head = 'head-b');
      await tester.pump();

      expect(await oldOpening, isFalse);
      final newOpening = controller.openAssetBySha256AtCheckpoint(
        _artifactSha,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-b',
      );
      headAOpen.complete(_fixture());
      await tester.pump();
      expect(find.byType(BottomSheet), findsNothing);
      headBOpen.complete(_fixture());
      await tester.pumpAndSettle();

      expect(await newOpening, isTrue);
      expect(find.byType(BottomSheet), findsOneWidget);
      expect(
        find.byKey(
          const ValueKey('revision3-content-asset-details-$_artifactSha'),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'controller buffers a pre-mount entity and opens its Problems section',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final controller = Revision3ContentLibraryController(
        projectIdentity: const Revision3ContentProjectIdentity(
          projectRoot: 'managed-root',
          projectId: _projectId,
        ),
      );
      bool? resolved;
      final opening = controller.openEntityProblemsById(_questId);
      unawaited(opening.then((value) => resolved = value));
      await tester.pump();
      expect(resolved, isNull, reason: 'the request waits for lazy mounting');

      await _pumpLoadedLibrary(tester, controller: controller);

      expect(await opening, isTrue);
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                Key(
                  'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
                ),
              ),
            )
            .selected,
        isTrue,
      );
      expect(
        find.byKey(
          Key(
            'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.byType(BottomSheet),
        findsNothing,
        reason:
            'wide Content keeps exact controller targets in its details pane',
      );
    },
  );

  testWidgets(
    'compact pre-mount Problems target opens its exact sheet without blocking',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      final controller = Revision3ContentLibraryController(
        projectIdentity: const Revision3ContentProjectIdentity(
          projectRoot: 'managed-root',
          projectId: _projectId,
        ),
      );
      bool? resolved;
      final opening = controller.openEntityProblemsById(_questId);
      unawaited(opening.then((value) => resolved = value));

      await _pumpLoadedLibrary(tester, controller: controller);

      expect(
        resolved,
        isTrue,
        reason: 'opening the modal must not wait for the user to dismiss it',
      );
      expect(find.byType(BottomSheet), findsOneWidget);
      expect(
        find.byKey(ValueKey('revision3-content-entity-details-$_questId')),
        findsOneWidget,
      );
      final problemsTab = find.byKey(
        Key(
          'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
        ),
      );
      expect(problemsTab, findsOneWidget);
      expect(tester.widget<ChoiceChip>(problemsTab).selected, isTrue);
      expect(
        find.byKey(
          Key(
            'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_questId',
          ),
        ),
        findsOneWidget,
      );
      expect(await opening, isTrue);
    },
  );

  testWidgets('compact controller asset target opens its exact sheet', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(tester, controller: controller);
    expect(find.byType(BottomSheet), findsNothing);

    bool? resolved;
    final opening = controller.openAssetBySha256(_artifactSha);
    unawaited(opening.then((value) => resolved = value));
    await tester.pumpAndSettle();

    expect(resolved, isTrue);
    expect(find.byType(BottomSheet), findsOneWidget);
    expect(
      find.byKey(ValueKey('revision3-content-asset-details-$_artifactSha')),
      findsOneWidget,
    );
    expect(await opening, isTrue);
  });

  testWidgets('same-project remount buffers and opens an exact compact target', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    final controller = Revision3ContentLibraryController(
      projectIdentity: const Revision3ContentProjectIdentity(
        projectRoot: 'managed-root',
        projectId: _projectId,
      ),
    );
    await _pumpLoadedLibrary(tester, controller: controller);

    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    await tester.pumpAndSettle();
    bool? resolved;
    final opening = controller.openEntityProblemsById(_npcId);
    unawaited(opening.then((value) => resolved = value));
    await tester.pump();
    expect(
      resolved,
      isNull,
      reason: 'a detached same-project request waits for the lazy remount',
    );

    await _pumpLoadedLibrary(tester, controller: controller);

    expect(resolved, isTrue);
    expect(find.byType(BottomSheet), findsOneWidget);
    expect(
      find.byKey(ValueKey('revision3-content-entity-details-$_npcId')),
      findsOneWidget,
    );
    final problemsTab = find.byKey(
      Key(
        'revision3-story-workbench-tab-${Revision3StoryWorkbenchSection.problemsChecks.name}-$_npcId',
      ),
    );
    expect(problemsTab, findsOneWidget);
    expect(tester.widget<ChoiceChip>(problemsTab).selected, isTrue);
    expect(await opening, isTrue);
  });

  testWidgets('controller buffers a pre-mount exact asset target', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final opening = controller.openAssetBySha256(_artifactSha);

    await _pumpLoadedLibrary(tester, controller: controller);

    expect(await opening, isTrue);
    expect(
      find.byKey(ValueKey('revision3-content-asset-details-$_artifactSha')),
      findsOneWidget,
    );
    expect(
      controller.projectIdentity,
      const Revision3ContentProjectIdentity(
        projectRoot: 'managed-root',
        projectId: _projectId,
      ),
    );
  });

  testWidgets('controller reports a missing pre-mount exact target', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final opening = controller.openEntityById(_missingId);

    await _pumpLoadedLibrary(tester, controller: controller);

    expect(await opening, isFalse);
    expect(
      find.byKey(ValueKey('revision3-content-entity-details-$_missingId')),
      findsNothing,
    );
  });

  testWidgets('newest pre-mount controller target supersedes the older one', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final superseded = controller.openEntityById(_npcId);
    final opening = controller.openAssetBySha256(_assetSha);

    expect(await superseded, isFalse);
    await _pumpLoadedLibrary(tester, controller: controller);

    expect(await opening, isTrue);
    expect(
      find.byKey(ValueKey('revision3-content-asset-details-$_assetSha')),
      findsOneWidget,
    );
  });

  testWidgets('requested Story section must be supported by the exact entity', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(tester, controller: controller);
    expect(await controller.openEntityById(_questId), isTrue);
    await tester.pump();

    expect(
      await controller.openEntityById(
        _moduleId,
        storySection: Revision3StoryWorkbenchSection.problemsChecks,
      ),
      isFalse,
    );
    await tester.pump();
    expect(
      find.byKey(ValueKey('revision3-content-entity-details-$_questId')),
      findsOneWidget,
      reason: 'an unsupported target does not disturb the current selection',
    );
  });

  testWidgets('project-bound pre-mount controller rejects another project', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController(
      projectIdentity: const Revision3ContentProjectIdentity(
        projectRoot: 'another-root',
        projectId: _projectId,
      ),
    );
    final opening = controller.openEntityById(_questId);

    await _pumpLoadedLibrary(tester, controller: controller);

    expect(await opening, isFalse);
    final remountOpening = controller.openEntityById(_questId);
    bool? remountResult;
    remountOpening.then((resolved) => remountResult = resolved);
    await tester.pump();

    expect(
      remountResult,
      isNull,
      reason:
          'the rejected library must not consume a request for the bound project',
    );
    controller.dispose();
    expect(await remountOpening, isFalse);
    expect(tester.takeException(), isNull);
  });

  test(
    'disposing a pre-mount controller cancels and permanently closes it',
    () async {
      final controller = Revision3ContentLibraryController();
      final opening = controller.openEntityById(_questId);

      controller.dispose();

      expect(await opening, isFalse);
      expect(await controller.openAssetBySha256(_assetSha), isFalse);
    },
  );

  testWidgets('controller resolves pending navigation against reloaded index', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final reopen = Completer<Revision3ContentIndex>();
    var calls = 0;
    await _pumpLibrary(
      tester,
      controller: controller,
      load: () {
        calls++;
        return calls == 1 ? Future.value(_fixture()) : reopen.future;
      },
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-content-refresh')));
    await tester.pump();
    bool? result;
    unawaited(
      controller.openEntityById(_moduleId).then((value) => result = value),
    );
    await tester.pump();
    expect(result, isNull, reason: 'the old visible index is not reused');

    reopen.complete(_fixture());
    await tester.pumpAndSettle();
    expect(result, isTrue);
    expect(
      find.byKey(ValueKey('revision3-content-entity-details-$_moduleId')),
      findsOneWidget,
    );
  });

  testWidgets('controller reports a target removed by exact reopen', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final reopen = Completer<Revision3ContentIndex>();
    var calls = 0;
    await _pumpLibrary(
      tester,
      controller: controller,
      load: () {
        calls++;
        return calls == 1 ? Future.value(_fixture()) : reopen.future;
      },
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-content-refresh')));
    await tester.pump();
    final result = controller.openEntityById(_npcId);
    reopen.complete(_fixture(includeNpc: false));
    await tester.pumpAndSettle();

    expect(await result, isFalse);
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsNothing);
  });

  testWidgets('project switch cancels a pending controller target', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    final oldReopen = Completer<Revision3ContentIndex>();
    final newProjectOpen = Completer<Revision3ContentIndex>();
    var calls = 0;
    var root = 'managed-root-a';
    var head = 'head-a';
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
                projectRevision: 7,
                projectHeadCanonicalJson: head,
                controller: controller,
                load: () {
                  calls++;
                  return switch (calls) {
                    1 => Future.value(_fixture()),
                    2 => oldReopen.future,
                    _ => newProjectOpen.future,
                  };
                },
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    rebuild(() => head = 'head-b');
    await tester.pump();
    final pending = controller.openEntityById(_questId);
    rebuild(() => root = 'managed-root-b');
    await tester.pump();

    expect(await pending, isFalse);
    oldReopen.complete(_fixture());
    newProjectOpen.complete(_fixture());
    await tester.pumpAndSettle();
    expect(calls, 3);
    final oldProjectOpening = controller.openEntityById(_questId);
    bool? oldProjectResult;
    oldProjectOpening.then((resolved) => oldProjectResult = resolved);
    await tester.pump();
    expect(
      oldProjectResult,
      isNull,
      reason: 'a controller must keep waiting for its bound project',
    );
    controller.dispose();
    expect(await oldProjectOpening, isFalse);
  });

  testWidgets('controller replacement and dispose leave no stale binding', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final first = Revision3ContentLibraryController();
    final second = Revision3ContentLibraryController();
    var controller = first;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3ContentLibrary(
                projectRoot: 'managed-root',
                projectId: _projectId,
                projectRevision: 7,
                projectHeadCanonicalJson: 'head',
                controller: controller,
                load: () async => _fixture(),
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    rebuild(() => controller = second);
    await tester.pump();
    final firstRemount = first.openEntityById(_questId);
    bool? firstResult;
    firstRemount.then((resolved) => firstResult = resolved);
    await tester.pump();
    expect(
      firstResult,
      isNull,
      reason: 'the detached controller must not forward into its replacement',
    );
    expect(await second.openEntityById(_questId), isTrue);
    await tester.pump();

    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    final secondRemount = second.openEntityById(_npcId);
    bool? secondResult;
    secondRemount.then((resolved) => secondResult = resolved);
    await tester.pump();
    expect(secondResult, isNull);
    first.dispose();
    second.dispose();
    expect(await firstRemount, isFalse);
    expect(await secondRemount, isFalse);
    expect(tester.takeException(), isNull);
  });

  testWidgets('explicit controller dispose clears an attached binding', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3ContentLibraryController();
    await _pumpLoadedLibrary(tester, controller: controller);

    controller.dispose();

    expect(await controller.openEntityById(_questId), isFalse);
    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
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
    expect(find.text('4 entities / 2 assets / revision 8'), findsOneWidget);
  });
}

Future<void> _openWorkbenchSection(
  WidgetTester tester,
  Revision3StoryWorkbenchSection section,
  String entityId,
) async {
  final tab = find.byKey(
    Key('revision3-story-workbench-tab-${section.name}-$entityId'),
  );
  await tester.ensureVisible(tab);
  await tester.tap(tab);
  await tester.pumpAndSettle();
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pumpHomeContentViewportLibrary(WidgetTester tester) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Padding(
          // The canonical Home workspace leaves this exact content rectangle
          // after its primary navigation and project header at all three
          // regression viewports.
          padding: const EdgeInsets.only(left: 339, top: 358),
          child: Revision3ContentLibrary(
            projectRoot: 'managed-root',
            projectId: _projectId,
            projectRevision: 7,
            projectHeadCanonicalJson: 'canonical-head-7',
            load: () async => _fixture(),
            editQuestOutline: (index, quest) async {},
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _selectQuestFromViewportList(WidgetTester tester) async {
  final entityList = find.byKey(const Key('revision3-content-entity-list'));
  final scrollable = find.descendant(
    of: entityList,
    matching: find.byType(Scrollable),
  );
  final quest = find.byKey(
    Key('revision3-content-entity-$_questId'),
    skipOffstage: false,
  );
  expect(entityList, findsOneWidget);
  expect(scrollable, findsOneWidget);
  expect(quest, findsOneWidget);
  await tester.scrollUntilVisible(quest, 80, scrollable: scrollable);
  await tester.pumpAndSettle();
  expect(quest.hitTestable(), findsOneWidget);
  await tester.tap(quest.hitTestable());
  await tester.pumpAndSettle();
}

Future<void> _expectOverviewActionFullyInteractive(WidgetTester tester) async {
  final section = find.byKey(
    Key(
      'revision3-story-workbench-section-${Revision3StoryWorkbenchSection.overview.name}-$_questId',
    ),
    skipOffstage: false,
  );
  final scrollable = find.descendant(
    of: section,
    matching: find.byType(Scrollable),
    skipOffstage: false,
  );
  final action = find.byKey(
    Key('revision3-story-workbench-action-edit-overview-$_questId'),
    skipOffstage: false,
  );
  expect(section, findsOneWidget);
  expect(scrollable, findsOneWidget);
  expect(action, findsOneWidget);
  await tester.scrollUntilVisible(action, 80, scrollable: scrollable);
  await tester.pumpAndSettle();

  final actionRect = tester.getRect(action);
  final viewportRect = tester.getRect(scrollable);
  expect(
    viewportRect.intersect(actionRect),
    actionRect,
    reason: 'the entire primary action must be visible inside its section',
  );
  expect(action.hitTestable(), findsOneWidget);

  final listTile = find.descendant(of: action, matching: find.byType(ListTile));
  final inkWell = find.descendant(of: action, matching: find.byType(InkWell));
  expect(listTile, findsOneWidget);
  expect(tester.widget<ListTile>(listTile).enabled, isTrue);
  expect(inkWell, findsOneWidget);
  expect(tester.widget<InkWell>(inkWell).canRequestFocus, isTrue);
}

Future<void> _pumpLoadedLibrary(
  WidgetTester tester, {
  int questGeneratorVersion = 4,
  String questGeneratorId = 'gore-authoring.draft-quest-skeleton',
  Revision3QuestOutlineEditor? editQuestOutline,
  Revision3QuestContextEditor? editQuestContext,
  Revision3QuestTransitionsEditor? editQuestTransitions,
  Revision3NpcProfileEditor? editNpcProfile,
  Revision3QuestSourceInspector? inspectQuestSource,
  Revision3NpcSourceInspector? inspectNpcSource,
  Revision3StoryDraftOpener? openStoryDraftInStory,
  String? openStoryDraftInStoryDisabledReason,
  String openStoryDraftLabel = 'Open in Story',
  String openStoryDraftDescription =
      'Continue editing this draft in the canonical Story workspace.',
  String openStoryDraftFailureMessage =
      'Story could not be opened. The project was not changed.',
  String? editQuestContextDisabledReason,
  String? inspectQuestSourceDisabledReason,
  Revision3ContentLibraryController? controller,
}) async {
  await _pumpLibrary(
    tester,
    load: () async => _fixture(
      questGeneratorVersion: questGeneratorVersion,
      questGeneratorId: questGeneratorId,
    ),
    editQuestOutline: editQuestOutline,
    editQuestContext: editQuestContext,
    editQuestTransitions: editQuestTransitions,
    editNpcProfile: editNpcProfile,
    inspectQuestSource: inspectQuestSource,
    inspectNpcSource: inspectNpcSource,
    openStoryDraftInStory: openStoryDraftInStory,
    openStoryDraftInStoryDisabledReason: openStoryDraftInStoryDisabledReason,
    openStoryDraftLabel: openStoryDraftLabel,
    openStoryDraftDescription: openStoryDraftDescription,
    openStoryDraftFailureMessage: openStoryDraftFailureMessage,
    editQuestContextDisabledReason: editQuestContextDisabledReason,
    inspectQuestSourceDisabledReason: inspectQuestSourceDisabledReason,
    controller: controller,
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
  Revision3NpcProfileEditor? editNpcProfile,
  Revision3QuestSourceInspector? inspectQuestSource,
  Revision3NpcSourceInspector? inspectNpcSource,
  Revision3StoryDraftOpener? openStoryDraftInStory,
  String? openStoryDraftInStoryDisabledReason,
  String openStoryDraftLabel = 'Open in Story',
  String openStoryDraftDescription =
      'Continue editing this draft in the canonical Story workspace.',
  String openStoryDraftFailureMessage =
      'Story could not be opened. The project was not changed.',
  String? editQuestContextDisabledReason,
  String? inspectQuestSourceDisabledReason,
  Revision3ContentLibraryController? controller,
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
        editNpcProfile: editNpcProfile,
        inspectQuestSource: inspectQuestSource,
        inspectNpcSource: inspectNpcSource,
        openStoryDraftInStory: openStoryDraftInStory,
        openStoryDraftInStoryDisabledReason:
            openStoryDraftInStoryDisabledReason,
        openStoryDraftLabel: openStoryDraftLabel,
        openStoryDraftDescription: openStoryDraftDescription,
        openStoryDraftFailureMessage: openStoryDraftFailureMessage,
        editQuestContextDisabledReason: editQuestContextDisabledReason,
        inspectQuestSourceDisabledReason: inspectQuestSourceDisabledReason,
        controller: controller,
      ),
    ),
  ),
);

Revision3ContentIndex _fixture({
  int revision = 7,
  bool includeQuest = true,
  int questGeneratorVersion = 4,
  String questGeneratorId = 'gore-authoring.draft-quest-skeleton',
  bool includeNpc = true,
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
    if (includeNpc) 'npc_draft': 1,
    if (includeQuest) 'quest_draft': 1,
    if (includeQuest || includeNpc)
      'script_module': (includeQuest ? 1 : 0) + (includeNpc ? 1 : 0),
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
            'greeting_count': 0,
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'draft_script_module',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': _projectId,
              'entity_id': _npcModuleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[],
      },
    if (includeNpc)
      <String, Object?>{
        'id': _npcModuleId,
        'kind': 'script_module',
        'display_name': 'Gate Guard source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'owner': <String, Object?>{
            'project_id': _projectId,
            'entity_id': _npcId,
            'expected_kind': 'npc_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.logical-npc-clone-draft',
            'generator_version': 1,
            'module_namespace': 'PROJECT.NPCS.GATEGUARD',
            'module_relative_path': 'Project/Npcs/GateGuard.as',
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
              'entity_id': _npcId,
              'expected_kind': 'npc_draft',
            },
            'resolution': 'resolved',
          },
          <String, Object?>{
            'role': 'script_owner',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': _projectId,
              'entity_id': _npcId,
              'expected_kind': 'npc_draft',
            },
            'resolution': 'resolved',
          },
        ],
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
            'additional_objective_titles': <String>[
              'Inspect the old gate',
              'Report the secured gate',
            ],
            'objective_slots': <Object?>[1, 2, 3],
            'transcript_count': 0,
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
    if (includeQuest)
      <String, Object?>{
        'id': _moduleId,
        'kind': 'script_module',
        'display_name': 'Find Homer source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': questGeneratorId,
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
            'generator_id': questGeneratorId,
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
