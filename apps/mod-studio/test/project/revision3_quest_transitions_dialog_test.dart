import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_dialog.dart';

import '../support/revision3_quest_outline_fixture.dart';

void main() {
  testWidgets('shows a friendly Root and Objective table without identities', (
    tester,
  ) async {
    await _open(tester);

    expect(find.text('Edit Quest behavior'), findsOneWidget);
    expect(find.text('Main Quest'), findsOneWidget);
    expect(find.text('Ask Asghan about Homer'), findsOneWidget);
    expect(find.text('Inspect the old gate'), findsOneWidget);
    expect(find.text('Report the secured gate'), findsOneWidget);
    expect(find.text('Engine default'), findsNWidgets(10));
    expect(find.text('Not used'), findsNWidgets(5));
    expect(find.textContaining(revision3QuestOutlineQuestId), findsNothing);
    expect(find.textContaining(revision3QuestOutlineModuleId), findsNothing);
    expect(find.textContaining('AngelScript'), findsNothing);
    expect(
      find.textContaining('offline project checkpoint only'),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-transitions-save')),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets('sequential template publishes through the technical callback', (
    tester,
  ) async {
    Revision3QuestTransitionsEditTechnicalPlan? received;
    Revision3QuestTransitionsEditPublication? result;
    await _open(
      tester,
      publish: (plan) {
        received = plan;
        return _publication(plan.transitionPlan.contentSeal);
      },
      onResult: (value) => result = value,
    );

    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-sequential-template')),
    );
    await tester.pump();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-transitions-save')),
          )
          .onPressed,
      isNotNull,
    );

    await tester.tap(find.byKey(const Key('revision3-quest-transitions-save')));
    await tester.pumpAndSettle();

    expect(received, isNotNull);
    expect(received?.questId, revision3QuestOutlineQuestId);
    expect(received?.transitionPlan.transitions.first.effects, isEmpty);
    final rootStart = received!.transitionPlan.transitions.singleWhere(
      (transition) =>
          transition.node.kind ==
              AuthoringRevision3QuestTransitionNodeKind.root &&
          transition.edge == AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    expect(
      rootStart.effects.single.effect,
      AuthoringRevision3QuestTransitionEffectKindV1.start,
    );
    expect(result?.projectRevision, 8);
    expect(
      find.byKey(const Key('revision3-quest-transitions-dialog')),
      findsNothing,
    );
  });

  testWidgets('external trigger and optional conditions are independent', (
    tester,
  ) async {
    await _open(tester);
    final cell = find.byKey(
      const Key('revision3-quest-transitions-cell-root-availability'),
    );
    await tester.ensureVisible(cell);
    await tester.pumpAndSettle();
    await tester.tap(cell);
    await tester.pumpAndSettle();

    final external = find.byKey(
      const Key('revision3-quest-transition-external'),
    );
    expect(tester.widget<SwitchListTile>(external).value, isTrue);
    await tester.tap(external);
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-quest-transition-add-alternative')),
    );
    await tester.pump();
    expect(find.text('Alternative 1'), findsOneWidget);
    expect(tester.widget<SwitchListTile>(external).value, isFalse);

    await tester.tap(find.byKey(const Key('revision3-quest-transition-apply')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-transition-editor')),
      findsNothing,
    );
    expect(
      find.descendant(of: cell, matching: find.text('Configured')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-transitions-save')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('offline project preview runs a cascade and resets locally', (
    tester,
  ) async {
    await _open(tester);
    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-sequential-template')),
    );
    await tester.pump();
    final openPreview = find.byKey(
      const Key('revision3-quest-logic-preview-open'),
    );
    await tester.ensureVisible(openPreview);
    await tester.tap(openPreview);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-quest-logic-preview-dialog')),
      findsOneWidget,
    );
    expect(find.textContaining('does not run the engine'), findsOneWidget);
    expect(
      find.textContaining('five conservative, mutually exclusive'),
      findsOneWidget,
    );
    final rootStart = find.byKey(
      const Key('revision3-quest-logic-preview-trigger-root-start'),
    );
    await tester.ensureVisible(rootStart);
    await tester.tap(rootStart);
    await tester.pump();

    expect(
      tester
          .widget<Text>(
            find.byKey(
              const Key('revision3-quest-logic-preview-state-root-running'),
            ),
          )
          .data,
      'Yes',
    );
    expect(
      tester
          .widget<Text>(
            find.byKey(
              const Key(
                'revision3-quest-logic-preview-state-objective:1-running',
              ),
            ),
          )
          .data,
      'Yes',
    );
    expect(
      find.descendant(
        of: find.byKey(const Key('revision3-quest-logic-preview-timeline')),
        matching: find.textContaining('follow-up action'),
      ),
      findsOneWidget,
    );

    await tester.tap(
      find.byKey(const Key('revision3-quest-logic-preview-reset')),
    );
    await tester.pump();
    expect(
      tester
          .widget<Text>(
            find.byKey(
              const Key('revision3-quest-logic-preview-state-root-running'),
            ),
          )
          .data,
      '—',
    );

    await tester.tap(
      find.byKey(const Key('revision3-quest-logic-preview-close')),
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-transitions-save')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('invalid driver stays in the editor with a useful error', (
    tester,
  ) async {
    await _open(tester);
    final cell = find.byKey(
      const Key('revision3-quest-transitions-cell-root-start'),
    );
    await tester.ensureVisible(cell);
    await tester.pumpAndSettle();
    await tester.tap(cell);
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-quest-transition-external')),
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-transition-apply')));
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-quest-transition-editor')),
      findsOneWidget,
    );
    expect(
      find.textContaining('needs an external or condition driver'),
      findsOneWidget,
    );
  });

  testWidgets('project conflict asks for a refreshed Quest checkpoint', (
    tester,
  ) async {
    await _open(
      tester,
      publish: (_) => throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transitions_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT',
        message: 'stale project',
      ),
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-sequential-template')),
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-transitions-save')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('reopen the Quest from the refreshed library'),
      findsOneWidget,
    );
    expect(
      find.text('Review the highlighted Quest behavior and try again.'),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-quest-transitions-dialog')),
      findsOneWidget,
    );
  });

  testWidgets('dirty cancel requires explicit discard', (tester) async {
    await _open(tester);
    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-sequential-template')),
    );
    await tester.pump();

    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-cancel')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-transitions-discard-dialog')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-keep-editing')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-transitions-dialog')),
      findsOneWidget,
    );

    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-cancel')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-quest-transitions-discard')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-transitions-dialog')),
      findsNothing,
    );
  });
}

Future<void> _open(
  WidgetTester tester, {
  Revision3QuestTransitionsEditPublication Function(
    Revision3QuestTransitionsEditTechnicalPlan plan,
  )?
  publish,
  ValueChanged<Revision3QuestTransitionsEditPublication?>? onResult,
}) async {
  final fixture = Revision3QuestOutlineFixture();
  final index = fixture.contentIndex();
  final service = Revision3QuestTransitionsAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
        }) async => AuthoringRevision3QuestTransitionsSeed.forProject(
          currentProjectJson: fixture.projectJson,
          questId: questId,
          expectedQuestRevision: expectedQuestRevision,
          expectedModuleId: expectedModuleId,
          expectedModuleRevision: expectedModuleRevision,
        ),
    publishTechnicalPlan: ({required plan}) async =>
        publish?.call(plan) ?? _publication(plan.transitionPlan.contentSeal),
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: FilledButton(
            onPressed: () async {
              final result =
                  await showDialog<Revision3QuestTransitionsEditPublication>(
                    context: context,
                    builder: (_) => Revision3QuestTransitionsEditDialog(
                      index: index,
                      quest: index.entityById(revision3QuestOutlineQuestId)!,
                      service: service,
                    ),
                  );
              onResult?.call(result);
            },
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('Open'));
  await tester.pumpAndSettle();
}

Revision3QuestTransitionsEditPublication _publication(
  AuthoringDraftContentSeal seal,
) => Revision3QuestTransitionsEditPublication(
  projectId: revision3QuestOutlineProjectId,
  projectRevision: 8,
  questId: revision3QuestOutlineQuestId,
  moduleId: revision3QuestOutlineModuleId,
  questRevision: 5,
  moduleRevision: 6,
  transitionPlanSeal: seal,
);
