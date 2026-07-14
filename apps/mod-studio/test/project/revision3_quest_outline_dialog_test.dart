import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_dialog.dart';

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
  });
}

Future<void> _open(
  WidgetTester tester, {
  required Revision3QuestOutlineEditPublisher publish,
}) async {
  final index = Revision3QuestOutlineFixture().contentIndex();
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
              ),
            ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('Open'));
  await tester.pumpAndSettle();
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
