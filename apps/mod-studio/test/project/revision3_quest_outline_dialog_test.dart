import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_dialog.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

void main() {
  testWidgets(
    'prefills outline, hides technical IDs and disables a no-op save',
    (tester) async {
      var calls = 0;
      await _open(
        tester,
        publish: ({required input}) async {
          calls++;
          return _publication(input);
        },
      );

      expect(find.text('Find Homer'), findsNWidgets(2));
      expect(find.text('Ask Asghan about Homer'), findsOneWidget);
      expect(find.text('Inspect the old gate'), findsOneWidget);
      expect(find.text('Report the secured gate'), findsOneWidget);
      expect(find.textContaining('Build remains blocked'), findsOneWidget);
      expect(
        find.textContaining('runtime behavior remains unqualified'),
        findsOneWidget,
      );
      expect(find.textContaining('3 existing objectives'), findsOneWidget);
      expect(find.textContaining('PROJECT.QUESTS'), findsNothing);
      expect(find.textContaining('UQuest_SwampCamp'), findsNothing);
      expect(find.textContaining('OM_GRD_Asghan'), findsNothing);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-quest-outline-save')),
            )
            .onPressed,
        isNull,
      );
      expect(
        find.byKey(const Key('revision3-quest-outline-objective-add')),
        findsNothing,
      );
      expect(find.byIcon(Icons.delete_outline), findsNothing);
      expect(calls, 0);
    },
  );

  testWidgets('renames, edits and reorders the fixed objective list once', (
    tester,
  ) async {
    Revision3QuestOutlineEditInput? received;
    await _open(
      tester,
      publish: ({required input}) async {
        received = input;
        return _publication(input);
      },
    );

    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-display-name')),
      'Find Homer safely',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-title')),
      'Find Homer safely',
    );
    await tester.pump();
    final moveDown = find.byKey(
      const Key('revision3-quest-outline-objective-down-0'),
    );
    await tester.ensureVisible(moveDown);
    await tester.pump();
    await tester.tap(moveDown);
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-objective-2')),
      'Report to Diego',
    );
    await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
    await tester.pumpAndSettle();

    expect(received, isNotNull);
    expect(received!.displayName, 'Find Homer safely');
    expect(received!.title, 'Find Homer safely');
    expect(received!.objectiveTitles, <String>[
      'Inspect the old gate',
      'Ask Asghan about Homer',
      'Report to Diego',
    ]);
    expect(received!.objectiveTitles, hasLength(3));
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsNothing,
    );
  });

  testWidgets('cancel makes zero publication calls', (tester) async {
    var calls = 0;
    await _open(
      tester,
      publish: ({required input}) async {
        calls++;
        return _publication(input);
      },
    );

    await tester.tap(find.byKey(const Key('revision3-quest-outline-cancel')));
    await tester.pumpAndSettle();

    expect(calls, 0);
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsNothing,
    );
  });

  testWidgets(
    'dirty cancel, barrier, Escape, and Back require explicit discard',
    (tester) async {
      await _open(
        tester,
        publish: ({required input}) async => _publication(input),
      );
      final title = find.byKey(const Key('revision3-quest-outline-title'));
      await tester.enterText(title, 'Do not lose this Quest outline');
      await tester.pump();

      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-outline-discard-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-quest-outline-keep-editing')),
      );
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(title).controller?.text,
        'Do not lose this Quest outline',
      );

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-outline-discard-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-quest-outline-keep-editing')),
      );
      await tester.pumpAndSettle();

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-outline-discard-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-quest-outline-keep-editing')),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-quest-outline-cancel')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-quest-outline-discard')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-outline-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('identity loading and publication cannot dismiss the editor', (
    tester,
  ) async {
    final fixture = Revision3QuestOutlineFixture();
    final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
      currentProjectJson: fixture.semanticProjectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
    );
    final seedCompletion = Completer<AuthoringRevision3QuestTransitionsSeed>();
    final publication = Completer<Revision3QuestOutlineEditPublication>();
    var publishes = 0;
    await _open(
      tester,
      semantic: true,
      settle: false,
      loadTransitionSeed:
          ({
            required questId,
            required expectedQuestRevision,
            required expectedModuleId,
            required expectedModuleRevision,
          }) => seedCompletion.future,
      publish: ({required input}) {
        publishes++;
        return publication.future;
      },
    );
    await tester.pump();

    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-quest-outline-cancel')),
          )
          .onPressed,
      isNull,
    );
    await tester.tapAt(const Offset(4, 4));
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsOneWidget,
    );

    seedCompletion.complete(seed);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-title')),
      'Find Homer safely',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
    await tester.pump();
    expect(publishes, 1);
    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-quest-outline-cancel')),
          )
          .onPressed,
      isNull,
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.tapAt(const Offset(4, 4));
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsOneWidget,
    );

    publication.complete(
      Revision3QuestOutlineEditPublication(
        projectId: revision3QuestOutlineProjectId,
        projectRevision: 8,
        questId: revision3QuestOutlineQuestId,
        moduleId: revision3QuestOutlineModuleId,
        questRevision: fixture.questRevision + 1,
        moduleRevision: fixture.moduleRevision + 1,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsNothing,
    );
  });

  testWidgets('stale checkpoint gives plain guidance and locks resubmit', (
    tester,
  ) async {
    var calls = 0;
    await _open(
      tester,
      publish: ({required input}) async {
        calls++;
        throw const Revision3QuestOutlineStaleCheckpointException();
      },
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-title')),
      'Find Homer safely',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(
      find.textContaining('project changed while this editor was open'),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-outline-save')),
          )
          .onPressed,
      isNull,
    );
    expect(find.text('Close'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-quest-outline-cancel')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-outline-discard-dialog')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsNothing,
    );
  });

  testWidgets('requires-reopen lock closes without a discard prompt', (
    tester,
  ) async {
    await _open(
      tester,
      publish: ({required input}) async =>
          throw const Revision3QuestOutlineRequiresReopenException(),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-outline-title')),
      'Find Homer safely',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
    await tester.pumpAndSettle();

    expect(find.textContaining('can no longer be verified'), findsOneWidget);
    expect(find.text('Close'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-quest-outline-cancel')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-outline-discard-dialog')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-quest-outline-dialog')),
      findsNothing,
    );
  });

  testWidgets(
    'semantic Quest reorders stable slots without losing behavior identity',
    (tester) async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
        currentProjectJson: fixture.semanticProjectJson,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
      );
      Revision3QuestOutlineEditInput? received;
      await _open(
        tester,
        semantic: true,
        loadTransitionSeed:
            ({
              required questId,
              required expectedQuestRevision,
              required expectedModuleId,
              required expectedModuleRevision,
            }) async => seed,
        publish: ({required input}) async {
          received = input;
          return _publication(input);
        },
      );

      expect(
        find.byKey(const Key('revision3-quest-outline-loading-identities')),
        findsNothing,
      );
      final moveDown = find.byKey(
        const Key('revision3-quest-outline-objective-down-0'),
      );
      expect(
        tester.widget<IconButton>(moveDown).onPressed,
        isNotNull,
        reason: tester
            .widgetList<Text>(find.byType(Text))
            .map((text) => text.data)
            .whereType<String>()
            .join(' | '),
      );
      await tester.ensureVisible(moveDown);
      await tester.pump();
      await tester.tap(moveDown);
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('revision3-quest-outline-objective-0')),
        'Inspect the secured gate',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
      await tester.pumpAndSettle();

      expect(received, isNotNull);
      expect(received!.usesStableObjectiveSlots, isTrue);
      expect(received!.objectiveSlots, [2, 1, 3]);
      expect(received!.objectiveTitles, [
        'Inspect the secured gate',
        'Ask Asghan about Homer',
        'Report the secured gate',
      ]);
      expect(
        received!.expectedTransitionPlanSeal?.sha256,
        seed.transitionPlanSeal.sha256,
      );
    },
  );

  testWidgets(
    'failed identity load keeps objectives locked until retry succeeds',
    (tester) async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
        currentProjectJson: fixture.semanticProjectJson,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
      );
      var loadCalls = 0;
      await _open(
        tester,
        semantic: true,
        loadTransitionSeed:
            ({
              required questId,
              required expectedQuestRevision,
              required expectedModuleId,
              required expectedModuleRevision,
            }) async {
              loadCalls++;
              if (loadCalls == 1) throw StateError('fixture load failure');
              return seed;
            },
        publish: ({required input}) async => _publication(input),
      );

      expect(loadCalls, 1);
      expect(
        find.textContaining('objective identities could not be loaded'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<TextField>(
              find.byKey(const Key('revision3-quest-outline-objective-0')),
            )
            .enabled,
        isFalse,
      );
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const Key('revision3-quest-outline-objective-down-0')),
            )
            .onPressed,
        isNull,
      );

      final retry = find.byKey(
        const Key('revision3-quest-outline-retry-identities'),
      );
      await tester.ensureVisible(retry);
      await tester.pump();
      await tester.tap(retry);
      await tester.pumpAndSettle();

      expect(loadCalls, 2);
      expect(
        tester
            .widget<TextField>(
              find.byKey(const Key('revision3-quest-outline-objective-0')),
            )
            .enabled,
        isTrue,
      );
      expect(
        find.textContaining(
          'Reordering keeps each objective identity and its behavior connections intact',
        ),
        findsOneWidget,
      );
    },
  );
}

Future<void> _open(
  WidgetTester tester, {
  required Revision3QuestOutlineEditPublisher publish,
  Revision3QuestTransitionsSeedLoader? loadTransitionSeed,
  bool semantic = false,
  bool settle = true,
}) async {
  final index = Revision3QuestOutlineFixture().contentIndex(
    questGeneratorVersion: semantic ? 4 : 3,
  );
  final quest = index.entityById(revision3QuestOutlineQuestId)!;
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: FilledButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => Revision3QuestOutlineEditDialog(
                index: index,
                quest: quest,
                publish: publish,
                loadTransitionSeed: loadTransitionSeed,
              ),
            ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('Open'));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

Revision3QuestOutlineEditPublication _publication(
  Revision3QuestOutlineEditInput input,
) => Revision3QuestOutlineEditPublication(
  projectId: revision3QuestOutlineProjectId,
  projectRevision: 8,
  questId: input.questId,
  moduleId: input.moduleId,
  questRevision: input.expectedQuestRevision + 1,
  moduleRevision: input.expectedModuleRevision + 1,
);
