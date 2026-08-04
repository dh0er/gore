import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_story_entity_workbench.dart';

const _projectA = '11111111111111111111111111111111';
const _questId = '22222222222222222222222222222222';
const _npcId = '33333333333333333333333333333333';
const _moduleId = '44444444444444444444444444444444';
const _npcModuleId = '2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _collisionSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
const _collisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';

void main() {
  test('Quest and NPC expose four canonical productive sections', () {
    final index = _fixture();
    final quest = index.entityById(_questId)!;
    final npc = index.entityById(_npcId)!;

    expect(
      Revision3StoryEntityWorkbench.sectionsFor(quest),
      const <Revision3StoryWorkbenchSection>[
        Revision3StoryWorkbenchSection.overview,
        Revision3StoryWorkbenchSection.dialogVoice,
        Revision3StoryWorkbenchSection.references,
        Revision3StoryWorkbenchSection.problemsChecks,
      ],
    );
    expect(
      Revision3StoryEntityWorkbench.sectionsFor(npc),
      const <Revision3StoryWorkbenchSection>[
        Revision3StoryWorkbenchSection.profile,
        Revision3StoryWorkbenchSection.dialogVoice,
        Revision3StoryWorkbenchSection.references,
        Revision3StoryWorkbenchSection.problemsChecks,
      ],
    );
    for (final section in Revision3StoryEntityWorkbench.sectionsFor(quest)) {
      expect(
        Revision3StoryEntityWorkbench.supportsSection(quest, section),
        isTrue,
      );
    }
    for (final section in Revision3StoryEntityWorkbench.sectionsFor(npc)) {
      expect(
        Revision3StoryEntityWorkbench.supportsSection(npc, section),
        isTrue,
      );
    }
  });

  testWidgets('wide Quest workbench exposes only the canonical four tabs', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final index = _fixture();

    await _pumpWorkbench(
      tester,
      index: index,
      questJourney: const Center(child: Text('Canonical Quest Journey')),
    );

    expect(
      find.byKey(Key('revision3-story-workbench-tab-overview-$_questId')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-dialogVoice-$_questId')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-references-$_questId')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-problemsChecks-$_questId')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-story-$_questId')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-logic-$_questId')),
      findsNothing,
    );
    expect(find.text('Canonical Quest Journey'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('fallback Overview invokes both Quest context editors', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final index = _fixture();
    var storyCalls = 0;
    var logicCalls = 0;

    await _pumpWorkbench(
      tester,
      index: index,
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editStory: () async => storyCalls++,
        editLogic: () async => logicCalls++,
      ),
    );

    final story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    final logic = _actionTile(
      'revision3-story-workbench-action-edit-logic-$_questId',
    );
    expect(story, findsOneWidget);
    expect(logic, findsOneWidget);
    expect(find.text('Edit description & connections'), findsOneWidget);
    expect(find.text('Edit states & transitions'), findsOneWidget);

    await tester.tap(story);
    await tester.pumpAndSettle();
    await tester.tap(logic);
    await tester.pumpAndSettle();

    expect(storyCalls, 1);
    expect(logicCalls, 1);
  });

  testWidgets('fallback Quest context editors are mutually single-flight', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final index = _fixture();
    final pendingStory = Completer<void>();
    var overviewCalls = 0;
    var storyCalls = 0;
    var logicCalls = 0;

    await _pumpWorkbench(
      tester,
      index: index,
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editOverview: () async => overviewCalls++,
        editStory: () {
          storyCalls++;
          return pendingStory.future;
        },
        editLogic: () async => logicCalls++,
      ),
    );

    final story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    final logic = _actionTile(
      'revision3-story-workbench-action-edit-logic-$_questId',
    );
    final overview = _actionTile(
      'revision3-story-workbench-action-edit-overview-$_questId',
    );
    final staleOverviewTap = tester.widget<ListTile>(overview).onTap!;
    final staleStoryTap = tester.widget<ListTile>(story).onTap!;
    final staleLogicTap = tester.widget<ListTile>(logic).onTap!;

    await tester.tap(story);
    await tester.pump();

    expect(overviewCalls, 0);
    expect(storyCalls, 1);
    expect(logicCalls, 0);
    expect(tester.widget<ListTile>(overview).enabled, isFalse);
    expect(tester.widget<ListTile>(story).enabled, isFalse);
    expect(tester.widget<ListTile>(logic).enabled, isFalse);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    staleOverviewTap();
    staleStoryTap();
    staleLogicTap();
    await tester.pump();
    expect(overviewCalls, 0);
    expect(storyCalls, 1);
    expect(logicCalls, 0);

    pendingStory.complete();
    await tester.pumpAndSettle();
    expect(tester.widget<ListTile>(overview).enabled, isTrue);
    expect(tester.widget<ListTile>(story).enabled, isTrue);
    expect(tester.widget<ListTile>(logic).enabled, isTrue);

    await tester.tap(logic);
    await tester.pumpAndSettle();
    expect(logicCalls, 1);
  });

  testWidgets('wide fallback failure is sanitized and can be retried', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final index = _fixture();
    var calls = 0;

    await _pumpWorkbench(
      tester,
      index: index,
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editStory: () async {
          calls++;
          throw StateError('private compiler path C:/secret/build.txt');
        },
      ),
    );

    final story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    await tester.tap(story);
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(tester.widget<ListTile>(story).enabled, isTrue);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(
      find.text('Could not open this editor. Please try again.'),
      findsOneWidget,
    );
    expect(find.textContaining('C:/secret/build.txt'), findsNothing);
    expect(tester.takeException(), isNull);

    await tester.tap(story);
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(tester.widget<ListTile>(story).enabled, isTrue);
  });

  testWidgets('compact fallback uses localized sanitized failure copy', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    final index = _fixture();

    await _pumpWorkbench(
      tester,
      index: index,
      copy: const Revision3StoryEntityWorkbenchCopy.english(
        actionFailed: 'Editor konnte nicht geoeffnet werden.',
      ),
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editStory: () async {
          throw ArgumentError('sensitive runtime signature 0xDEADBEEF');
        },
      ),
    );

    final story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    await tester.tap(story);
    await tester.pumpAndSettle();

    expect(find.text('Editor konnte nicht geoeffnet werden.'), findsOneWidget);
    expect(find.textContaining('0xDEADBEEF'), findsNothing);
    expect(tester.widget<ListTile>(story).enabled, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('compact fallback exposes exact disabled editor reasons', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    final index = _fixture();

    await _pumpWorkbench(
      tester,
      index: index,
      actions: const Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editStoryDisabledReason: 'Story lease unavailable.',
        editLogicDisabledReason: 'Logic lease unavailable.',
      ),
    );

    final story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    expect(tester.widget<ListTile>(story).enabled, isFalse);
    expect(find.text('Story lease unavailable.'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.byKey(Key('revision3-story-workbench-action-edit-logic-$_questId')),
      160,
      scrollable: find.descendant(
        of: find.byKey(
          Key('revision3-story-workbench-section-overview-$_questId'),
        ),
        matching: find.byType(Scrollable),
      ),
    );
    await tester.pumpAndSettle();
    final logic = _actionTile(
      'revision3-story-workbench-action-edit-logic-$_questId',
    );

    expect(tester.widget<ListTile>(logic).enabled, isFalse);
    expect(find.text('Logic lease unavailable.'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('canonical Journey is the sole owner of context handoffs', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final index = _fixture();

    await _pumpWorkbench(
      tester,
      index: index,
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: _ignoreString,
        openAsset: _ignoreString,
        editStory: () async {},
        editLogic: () async {},
      ),
      questJourney: const Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('Edit description & connections'),
            Text('Edit states & transitions'),
          ],
        ),
      ),
    );

    expect(
      find.byKey(Key('revision3-story-workbench-action-edit-story-$_questId')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-action-edit-logic-$_questId')),
      findsNothing,
    );
    expect(find.text('Edit description & connections'), findsOneWidget);
    expect(find.text('Edit states & transitions'), findsOneWidget);
  });

  testWidgets('compact Quest workbench scrolls and opens Dialog & Voice', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    final index = _fixture();
    final dialogTab = find.byKey(
      Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
    );

    await _pumpWorkbench(
      tester,
      index: index,
      questJourney: const Center(child: Text('Canonical Quest Journey')),
      questTranscript: const Text('Exact Quest transcript'),
    );

    expect(
      find.byKey(Key('revision3-story-workbench-tabs-$_questId')),
      findsOneWidget,
    );
    expect(dialogTab, findsOneWidget);
    await tester.ensureVisible(dialogTab);
    await tester.tap(dialogTab);
    await tester.pumpAndSettle();

    expect(
      find.byKey(
        Key('revision3-story-workbench-section-dialogVoice-$_questId'),
      ),
      findsOneWidget,
    );
    expect(find.text('Exact Quest transcript'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('NPC Dialog & Voice hosts an exact supplied greeting editor', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(760, 720));
    final index = _fixture();
    await _pumpWorkbench(
      tester,
      index: index,
      entityId: _npcId,
      selectedSection: Revision3StoryWorkbenchSection.profile,
      npcDialogVoice: const Text('Exact NPC greeting editor'),
    );

    final dialogTab = find.byKey(
      Key('revision3-story-workbench-tab-dialogVoice-$_npcId'),
    );
    await tester.ensureVisible(dialogTab);
    await tester.tap(dialogTab);
    await tester.pumpAndSettle();

    expect(find.text('Exact NPC greeting editor'), findsOneWidget);
    expect(find.text('Not modeled yet'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'NPC Profile is one productive Character journey with planned work collapsed',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1000, 720));
      final index = _fixture();
      var profileEdits = 0;

      await _pumpWorkbench(
        tester,
        index: index,
        entityId: _npcId,
        selectedSection: Revision3StoryWorkbenchSection.profile,
        npcDialogVoice: const Text('Exact NPC greeting editor'),
        actions: Revision3StoryEntityWorkbenchActions(
          openEntity: _ignoreString,
          openAsset: _ignoreString,
          editNpcProfile: () async => profileEdits++,
        ),
      );

      for (final section in const <Revision3StoryWorkbenchSection>[
        Revision3StoryWorkbenchSection.profile,
        Revision3StoryWorkbenchSection.dialogVoice,
        Revision3StoryWorkbenchSection.references,
        Revision3StoryWorkbenchSection.problemsChecks,
      ]) {
        expect(
          find.byKey(
            Key('revision3-story-workbench-tab-${section.name}-$_npcId'),
          ),
          findsOneWidget,
        );
      }
      for (final removedSection in const <String>[
        'story',
        'routine',
        'inventory',
      ]) {
        expect(
          find.byKey(
            Key('revision3-story-workbench-tab-$removedSection-$_npcId'),
          ),
          findsNothing,
        );
      }

      expect(find.text('Gate Guard'), findsWidgets);
      expect(find.text('Next step: Dialog & Voice'), findsOneWidget);
      expect(
        find.byKey(
          const Key('revision3-story-workbench-npc-planned-capabilities'),
        ),
        findsOneWidget,
      );
      expect(
        find.text(
          'Quest and story relationships are not modeled for NPC drafts yet.',
        ),
        findsNothing,
        reason: 'planned domains start collapsed instead of filling tabs',
      );
      expect(
        find.byKey(
          const ValueKey('revision3-story-workbench-unavailable-Story'),
        ),
        findsNothing,
        reason: 'the old standalone empty capability is gone',
      );

      await tester.tap(
        _actionTile(
          'revision3-story-workbench-action-edit-npc-profile-$_npcId',
        ),
      );
      await tester.pumpAndSettle();
      expect(profileEdits, 1);

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
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'compact scaled NPC profile continues to Dialog & Voice for the same draft',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 640));
      final index = _fixture();
      final sectionChanges = <Revision3StoryWorkbenchSection>[];

      await tester.pumpWidget(
        MaterialApp(
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: const TextScaler.linear(2)),
            child: child!,
          ),
          home: Scaffold(
            body: Revision3StoryEntityWorkbench(
              projectId: index.projectId,
              index: index,
              entity: index.entityById(_npcId)!,
              selectedSection: Revision3StoryWorkbenchSection.profile,
              onSectionChanged: sectionChanges.add,
              actions: _actions,
              npcDialogVoice: const Text('Exact NPC greeting editor'),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final details = find.byKey(
        Key('revision3-content-entity-details-$_npcId'),
      );
      final profile = find.byKey(
        Key('revision3-story-workbench-section-profile-$_npcId'),
      );
      final nextStep = find.byKey(
        Key('revision3-story-workbench-npc-dialog-next-step-$_npcId'),
      );
      final continueButton = find.descendant(
        of: nextStep,
        matching: find.widgetWithText(
          FilledButton,
          'Continue to Dialog & Voice',
        ),
      );

      expect(details, findsOneWidget);
      expect(profile, findsOneWidget);
      expect(tester.takeException(), isNull);

      await tester.scrollUntilVisible(
        nextStep,
        120,
        scrollable: find.descendant(
          of: profile,
          matching: find.byType(Scrollable),
        ),
      );
      await tester.pumpAndSettle();

      expect(nextStep, findsOneWidget);
      expect(find.text('Next step: Dialog & Voice'), findsOneWidget);
      expect(
        find.textContaining('does not create playable dialog'),
        findsOneWidget,
      );
      expect(
        find.descendant(of: nextStep, matching: find.textContaining(_npcId)),
        findsNothing,
      );
      expect(
        find.descendant(
          of: nextStep,
          matching: find.textContaining('GORE_GATE_GUARD'),
        ),
        findsNothing,
      );
      expect(tester.takeException(), isNull);

      final profileScroll = find.descendant(
        of: profile,
        matching: find.byType(Scrollable),
      );
      for (
        var attempt = 0;
        attempt < 20 && continueButton.hitTestable().evaluate().isEmpty;
        attempt++
      ) {
        await tester.drag(profileScroll, const Offset(0, -120));
        await tester.pump();
      }
      expect(continueButton.hitTestable(), findsOneWidget);
      await tester.tap(continueButton.hitTestable());
      await tester.pumpAndSettle();

      expect(sectionChanges, const <Revision3StoryWorkbenchSection>[
        Revision3StoryWorkbenchSection.dialogVoice,
      ]);
      expect(details, findsOneWidget);
      expect(profile, findsNothing);
      expect(
        find.byKey(
          Key('revision3-story-workbench-section-dialogVoice-$_npcId'),
        ),
        findsOneWidget,
      );
      expect(find.text('Exact NPC greeting editor'), findsOneWidget);
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                Key('revision3-story-workbench-tab-dialogVoice-$_npcId'),
              ),
            )
            .selected,
        isTrue,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'compact scaled NPC planned summary expands accessibly without overflow',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 640));
      final index = _fixture();

      await tester.pumpWidget(
        MaterialApp(
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: const TextScaler.linear(2)),
            child: child!,
          ),
          home: Scaffold(
            body: Revision3StoryEntityWorkbench(
              projectId: index.projectId,
              index: index,
              entity: index.entityById(_npcId)!,
              selectedSection: Revision3StoryWorkbenchSection.profile,
              onSectionChanged: (_) {},
              actions: _actions,
              npcDialogVoice: const Text('Exact NPC greeting editor'),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final profile = find.byKey(
        Key('revision3-story-workbench-section-profile-$_npcId'),
      );
      final profileScroll = find.descendant(
        of: profile,
        matching: find.byType(Scrollable),
      );
      final plannedTitle = find.text('Story, Routine, Inventory');
      await tester.scrollUntilVisible(
        plannedTitle,
        120,
        scrollable: profileScroll,
      );
      await tester.pumpAndSettle();

      final semantics = tester.ensureSemantics();
      final plannedSemantics = find.semantics.byLabel(
        RegExp('Story, Routine, Inventory'),
      );
      expect(plannedSemantics, findsOneWidget);
      tester.semantics.tap(plannedSemantics);
      await tester.pumpAndSettle();

      expect(
        find.text('Routine and world placement are not modeled yet.'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets('same-project entity revisions retain a valid selected tab', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 720));
    var index = _fixture(revision: 7, entityRevision: 1);
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              final quest = index.entityById(_questId)!;
              return Revision3StoryEntityWorkbench(
                projectId: index.projectId,
                index: index,
                entity: quest,
                selectedSection: Revision3StoryWorkbenchSection.overview,
                onSectionChanged: (_) {},
                actions: _actions,
                questJourney: const Center(
                  child: Text('Canonical Quest Journey'),
                ),
                questTranscript: Text(
                  'Exact Quest transcript r${quest.revision}',
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final dialogTab = find.byKey(
      Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
    );
    await tester.tap(dialogTab);
    await tester.pumpAndSettle();
    expect(find.text('Exact Quest transcript r1'), findsOneWidget);

    rebuild(() => index = _fixture(revision: 8, entityRevision: 2));
    await tester.pumpAndSettle();

    expect(tester.widget<ChoiceChip>(dialogTab).selected, isTrue);
    expect(find.text('Exact Quest transcript r2'), findsOneWidget);
    expect(find.text('Canonical Quest Journey'), findsNothing);
  });

  testWidgets('new exact revision releases only its own fallback action lane', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 720));
    var index = _fixture(revision: 7, entityRevision: 1);
    final oldPending = Completer<void>();
    final currentPending = Completer<void>();
    var oldCalls = 0;
    var currentCalls = 0;
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              final quest = index.entityById(_questId)!;
              return Revision3StoryEntityWorkbench(
                projectId: index.projectId,
                index: index,
                entity: quest,
                selectedSection: Revision3StoryWorkbenchSection.overview,
                onSectionChanged: (_) {},
                actions: Revision3StoryEntityWorkbenchActions(
                  openEntity: _ignoreString,
                  openAsset: _ignoreString,
                  editStory: () {
                    if (quest.revision == 1) {
                      oldCalls++;
                      return oldPending.future;
                    }
                    currentCalls++;
                    return currentPending.future;
                  },
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    var story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    final staleTap = tester.widget<ListTile>(story).onTap!;
    await tester.tap(story);
    await tester.pump();
    expect(oldCalls, 1);
    expect(tester.widget<ListTile>(story).enabled, isFalse);

    rebuild(() => index = _fixture(revision: 8, entityRevision: 2));
    await tester.pumpAndSettle();
    story = _actionTile(
      'revision3-story-workbench-action-edit-story-$_questId',
    );
    expect(tester.widget<ListTile>(story).enabled, isTrue);

    staleTap();
    await tester.pump();
    expect(oldCalls, 1, reason: 'stale action epoch must fail closed');

    await tester.tap(story);
    await tester.pump();
    expect(currentCalls, 1);
    expect(tester.widget<ListTile>(story).enabled, isFalse);

    oldPending.completeError(
      StateError('stale revision leaked compiler path C:/secret/stale.txt'),
    );
    await tester.pump();
    expect(
      tester.widget<ListTile>(story).enabled,
      isFalse,
      reason: 'old completion must not release the current exact lane',
    );
    expect(
      find.text('Could not open this editor. Please try again.'),
      findsNothing,
      reason: 'a stale action error must be completely inert',
    );
    expect(find.textContaining('C:/secret/stale.txt'), findsNothing);

    currentPending.complete();
    await tester.pumpAndSettle();
    expect(tester.widget<ListTile>(story).enabled, isTrue);
  });
}

const _actions = Revision3StoryEntityWorkbenchActions(
  openEntity: _ignoreString,
  openAsset: _ignoreString,
);

void _ignoreString(String _) {}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pumpWorkbench(
  WidgetTester tester, {
  required Revision3ContentIndex index,
  String entityId = _questId,
  Revision3StoryWorkbenchSection selectedSection =
      Revision3StoryWorkbenchSection.overview,
  Widget? questJourney,
  Widget? questTranscript,
  Widget? npcDialogVoice,
  Revision3StoryEntityWorkbenchActions actions = _actions,
  Revision3StoryEntityWorkbenchCopy copy =
      const Revision3StoryEntityWorkbenchCopy.english(),
}) async {
  final entity = index.entityById(entityId)!;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Revision3StoryEntityWorkbench(
          projectId: index.projectId,
          index: index,
          entity: entity,
          selectedSection: selectedSection,
          onSectionChanged: (_) {},
          actions: actions,
          questJourney: questJourney,
          questTranscript: questTranscript,
          npcDialogVoice: npcDialogVoice,
          copy: copy,
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Finder _actionTile(String key) =>
    find.descendant(of: find.byKey(Key(key)), matching: find.byType(ListTile));

Revision3ContentIndex _fixture({int revision = 7, int entityRevision = 1}) =>
    Revision3ContentIndex.fromJsonObject(<String, Object?>{
      'schema_revision': 1,
      'project_id': _projectA,
      'project_revision': revision,
      'project_name': 'Workbench fixture',
      'project_version': '0.1.0',
      'project_author': 'GORE',
      'target': <String, Object?>{
        'executable': <String, Object?>{'byte_len': 123, 'sha256': _targetSha},
      },
      'authoring_locales': <Object?>['de', 'en'],
      'entity_counts': <String, Object?>{
        'npc_draft': 1,
        'quest_draft': 1,
        'script_module': 2,
      },
      'entities': <Object?>[
        <String, Object?>{
          'id': _questId,
          'kind': 'quest_draft',
          'display_name': 'Find Homer',
          'revision': entityRevision,
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
              'objective_slots': <Object?>[1],
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
                'project_id': _projectA,
                'entity_id': _moduleId,
                'expected_kind': 'script_module',
              },
              'resolution': 'resolved',
            },
          ],
          'asset_references': <Object?>[
            <String, Object?>{
              'role': 'quest_collision_artifact',
              'sha256': _collisionSha,
              'byte_len': 123,
              'logical_name': null,
              'expected_media_type': _collisionMediaType,
              'resolution': 'resolved',
            },
          ],
        },
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
              'project_id': _projectA,
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
                'project_id': _projectA,
                'entity_id': _npcId,
                'expected_kind': 'npc_draft',
              },
              'resolution': 'resolved',
            },
            <String, Object?>{
              'role': 'script_owner',
              'qualifier': null,
              'target': <String, Object?>{
                'project_id': _projectA,
                'entity_id': _npcId,
                'expected_kind': 'npc_draft',
              },
              'resolution': 'resolved',
            },
          ],
          'asset_references': <Object?>[],
        },
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
                'project_id': _projectA,
                'entity_id': _npcModuleId,
                'expected_kind': 'script_module',
              },
              'resolution': 'resolved',
            },
          ],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': _moduleId,
          'kind': 'script_module',
          'display_name': 'Find Homer source',
          'revision': entityRevision,
          'origin': <String, Object?>{
            'type': 'generated',
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': 4,
            'owner': <String, Object?>{
              'project_id': _projectA,
              'entity_id': _questId,
              'expected_kind': 'quest_draft',
            },
          },
          'summary': <String, Object?>{
            'kind': 'script_module',
            'data': <String, Object?>{
              'generator_id': 'gore-authoring.draft-quest-skeleton',
              'generator_version': 4,
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
                'project_id': _projectA,
                'entity_id': _questId,
                'expected_kind': 'quest_draft',
              },
              'resolution': 'resolved',
            },
            <String, Object?>{
              'role': 'script_owner',
              'qualifier': null,
              'target': <String, Object?>{
                'project_id': _projectA,
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
          'sha256': _collisionSha,
          'byte_len': 123,
          'media_type': _collisionMediaType,
          'class': 'quest_collision_artifact',
        },
      ],
    });
