import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/shared_config.dart';
import 'package:gore_mod/app/domain/ui_settings.dart' show sharedConfigProvider;
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/dataasset/ui/dataasset_lab.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';
import 'package:gore_mod/gore_mod_app.dart';
import 'package:gore_mod/home_page.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_wizard.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

void main() {
  testWidgets(
    'managed open owns the shell, hides legacy actions, and Ctrl+S verifies',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final legacy = _FakeLegacyLease(path: 'legacy.goremod');
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\managed-r3'),
        projectId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        projectRevision: 23,
        head: _head(23),
      );
      Directory? requestedRoot;
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (root) async {
          requestedRoot = root;
          return managed;
        },
      );
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => managed.root.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      expect(find.text('Build / Deploy'), findsOneWidget);

      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('openManagedRevision3');
      for (var i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }

      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(requestedRoot?.path, managed.root.path);
      expect(
        find.byKey(const Key('managed-revision3-project-view')),
        findsOneWidget,
      );
      expect(find.text(managed.root.path), findsOneWidget);
      expect(find.text(managed.projectId), findsOneWidget);
      expect(find.text('${managed.projectRevision}'), findsOneWidget);
      expect(find.text(managed.head.snapshotSha256), findsOneWidget);
      expect(find.text('Build / Deploy'), findsNothing);
      expect(find.byType(TabBar), findsOneWidget);
      expect(
        find.byKey(const Key('managed-revision3-library-tab')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('managed-revision3-dataasset-tab')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('managed-revision3-dataasset-tab')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dataasset-stage-panel')),
        findsOneWidget,
      );
      expect(find.textContaining('not yet included in builds'), findsOneWidget);
      expect(managed.dataAssetListCalls, 1);
      expect(legacy.closeCalls, 1);

      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      await tester.tap(find.byKey(const Key('managed-open-settings')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('managed-settings-dialog')), findsOneWidget);
      await tester.tap(find.byKey(const Key('managed-settings-close')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('managed-settings-dialog')), findsNothing);

      final menuFinder = find.byKey(const Key('project-menu'));
      final menu = tester.widget<PopupMenuButton<String>>(menuFinder);
      final saveAs = menu
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .singleWhere((item) => item.key == const Key('project-save-as'));
      expect(saveAs.enabled, isFalse);
      expect((saveAs.child as Text).data, 'Save project as…');

      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyS);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pump(const Duration(milliseconds: 300));

      expect(managed.verifyCalls, 1);
      final verified = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(verified.head.canonicalJson, managed.head.canonicalJson);
    },
  );

  testWidgets(
    'visible managed Quest wizard publishes and reloads the new revision',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_quest_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '18181818181818181818181818181818';
      var catalogLoads = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-authoring'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onQuestPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.title, 'Find Homer');
          expect(input.parentCatalogId, 'parent-one');
          expect(input.giverCatalogId, 'giver-asghan');
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3QuestDraftPublication(
            projectId: projectId,
            projectRevision: 8,
            questId: '28282828282828282828282828282828',
            scriptModuleId: '38383838383838383838383838383838',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadQuestCatalog: (_) async {
          catalogLoads++;
          return _questCatalog();
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      final createButton = find.byKey(const Key('managed-create-quest-draft'));
      expect(createButton, findsOneWidget);
      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      expect(tester.widget<FilledButton>(createButton).onPressed, isNotNull);
      expect(managed.contentReadCalls, 1);

      await tester.tap(createButton);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-quest-wizard')), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-quest-title')),
        'Find Homer',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-description')),
        'Homer vanished near the old gate.',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-objective')),
        'Ask Asghan about Homer',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await tester.pumpAndSettle();

      expect(catalogLoads, 2);
      expect(managed.questPublishCalls, 1);
      expect(managed.contentReadCalls, 2);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(find.byKey(const Key('managed-project-revision')), findsOneWidget);
      expect(find.text('8'), findsWidgets);
      expect(
        find.textContaining('Quest draft saved in project revision 8'),
        findsOneWidget,
      );
      expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
    },
  );

  testWidgets(
    'selected Library Quest opens count-preserving outline edit and refreshes selection',
    (tester) async {
      await _setDesktopTestSurface(tester);
      var currentFixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-outline-authoring'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (_) => currentFixture.contentIndex(),
        onQuestOutlinePublish: (lease, input) {
          expect(input.questId, revision3QuestOutlineQuestId);
          expect(input.moduleId, revision3QuestOutlineModuleId);
          expect(input.objectiveTitles, currentFixture.objectiveTitles);
          lease.projectRevision = 8;
          lease.head = _head(8);
          currentFixture = Revision3QuestOutlineFixture(
            projectRevision: 8,
            questRevision: 5,
            moduleRevision: 6,
            displayName: input.displayName,
            title: input.title,
            objectiveTitles: input.objectiveTitles,
          );
          return Revision3QuestOutlineEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: 8,
            questId: input.questId,
            moduleId: input.moduleId,
            questRevision: 5,
            moduleRevision: 6,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      final edit = find.byKey(
        const Key(
          'revision3-content-edit-quest-outline-$revision3QuestOutlineQuestId',
        ),
      );
      if (edit.evaluate().isEmpty) {
        await tester.tap(
          find.byKey(
            const Key('revision3-content-entity-$revision3QuestOutlineQuestId'),
          ),
        );
        await tester.pumpAndSettle();
        if (edit.evaluate().isEmpty) {
          await tester.drag(
            find.byKey(const Key('revision3-content-entity-details')),
            const Offset(0, -300),
          );
          await tester.pump();
        }
      }
      expect(edit, findsOneWidget);
      await tester.tap(edit);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-outline-dialog')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-outline-objective-add')),
        findsNothing,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-outline-title')),
        'Find Homer safely',
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-quest-outline-save')));
      await tester.pumpAndSettle();

      expect(managed.questOutlinePublishCalls, 1);
      expect(managed.contentReadCalls, 2);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
      expect(currentFixture.title, 'Find Homer safely');
      final selectedQuest = find.byKey(
        const Key('revision3-content-entity-$revision3QuestOutlineQuestId'),
      );
      expect(
        tester.widget<ListTile>(selectedQuest).selected,
        isTrue,
        reason: 'the same Quest remains selected across the exact-head reload',
      );
      expect(
        find.textContaining(
          'Build remains blocked; runtime remains unqualified',
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'visible managed NPC wizard publishes and reloads the new revision',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync('gore_r3_npc_game');
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '19191919191919191919191919191919';
      var catalogLoads = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\npc-authoring'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.displayName, 'North Gate Guard');
          expect(input.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3NpcDraftPublication(
            projectId: projectId,
            projectRevision: 8,
            npcId: '29292929292929292929292929292929',
            scriptModuleId: '39393939393939393939393939393939',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadNpcCatalog: (_) async {
          catalogLoads++;
          return _npcCatalog();
        },
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      final createButton = find.byKey(const Key('managed-create-npc-draft'));
      expect(createButton, findsOneWidget);
      expect(tester.widget<FilledButton>(createButton).onPressed, isNotNull);
      expect(managed.contentReadCalls, 1);

      await tester.tap(createButton);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      expect(find.text('Asghan guard'), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'North Gate Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      expect(catalogLoads, 2);
      expect(managed.npcPublishCalls, 1);
      expect(managed.contentReadCalls, 2);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(find.text('8'), findsWidgets);
      expect(
        find.textContaining('NPC draft saved in project revision 8'),
        findsOneWidget,
      );
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets(
    'visible managed Voice wizard forwards the configured safety game root',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-authoring'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            revision3VoiceContentIndexFixture(revision: lease.projectRevision),
        onVoicePublish: (lease, requestedGameRoot, plan) {
          expect(requestedGameRoot, gameRoot.path);
          expect(plan.lineId, revision3VoiceContentLineId);
          expect(plan.slotId, revision3VoiceContentSlotId);
          expect(plan.logicalName, 'asghan.ogg');
          expect(plan.text, isNull);
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakePublication(
            projectId: lease.projectId,
            projectRevision: 8,
            lineId: plan.lineId,
            slotId: plan.slotId,
            takeId: plan.takeId,
            slotCreated: plan.expectsSlotCreated,
            selected: plan.selectTake,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      final voiceButton = find.byKey(const Key('managed-add-voice-take'));
      expect(voiceButton, findsOneWidget);
      expect(tester.widget<FilledButton>(voiceButton).onPressed, isNotNull);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('managed-create-npc-draft')),
            )
            .onPressed,
        isNotNull,
      );

      await tester.tap(voiceButton);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-voice-wizard')), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-voice-line-search')),
        'GRD_263_ASGHAN',
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.text(
          'Asghan — Mine entrance question · GRD_263_ASGHAN_OPEN_INFO_06_02',
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-voice-source')),
        r'C:\Voice\asghan.ogg',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-voice-take-name')),
        'Asghan take',
      );
      await tester.ensureVisible(
        find.byKey(const Key('revision3-voice-submit')),
      );
      await tester.tap(find.byKey(const Key('revision3-voice-submit')));
      await tester.pumpAndSettle();

      expect(managed.voicePublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(find.text('8'), findsWidgets);
      expect(
        find.textContaining('Voice take saved in project revision 8'),
        findsOneWidget,
      );
      expect(find.byKey(const Key('revision3-voice-wizard')), findsNothing);
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets(
    'managed Voice target and offline build use the exact current checkpoint',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_target_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      final buildParent = Directory.systemTemp.createTempSync(
        'gore_r3_voice_build_parent_',
      );
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
        if (buildParent.existsSync()) buildParent.deleteSync(recursive: true);
      });

      String? targetGameRoot;
      Revision3VoiceTargetTechnicalPlan? targetPlan;
      String? buildGameRoot;
      String? buildOutput;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-target-build'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            revision3VoiceContentIndexFixture(revision: lease.projectRevision),
        onVoiceTargetPublish: (lease, requestedGameRoot, plan) {
          targetGameRoot = requestedGameRoot;
          targetPlan = plan;
          expect(lease.projectRevision, 7);
          expect(lease.head.canonicalJson, _head(7).canonicalJson);
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTargetPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            lineId: plan.lineId,
            slotId: plan.slotId,
            locale: plan.locale,
            locId: plan.locId,
            resolution: AuthoringRevision3VoiceTargetResolutionState.resolved,
            matchCount: 1,
          );
        },
        onVoiceBuild: (lease, requestedGameRoot, output) {
          buildGameRoot = requestedGameRoot;
          buildOutput = output;
          expect(lease.projectRevision, 8);
          expect(lease.head.canonicalJson, _head(8).canonicalJson);
          return _builtVoiceResult(lease: lease, output: output);
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (label) async {
          expect(label, 'Choose Voice bundle parent');
          return buildParent.path;
        },
        gamePath: gameRoot.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      final voiceTools = find.byKey(const Key('managed-voice-tools'));
      final voiceToolsButton = tester.widget<PopupMenuButton<String>>(
        voiceTools,
      );
      expect(voiceToolsButton.enabled, isTrue);
      final voiceToolItems = voiceToolsButton
          .itemBuilder(tester.element(voiceTools))
          .whereType<PopupMenuItem<String>>()
          .toList(growable: false);
      expect(
        voiceToolItems
            .singleWhere(
              (item) => item.key == const Key('managed-resolve-voice-target'),
            )
            .enabled,
        isTrue,
      );
      expect(
        voiceToolItems
            .singleWhere(
              (item) => item.key == const Key('managed-build-voice-bundle'),
            )
            .enabled,
        isTrue,
      );

      await tester.tap(voiceTools);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-resolve-voice-target')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-voice-target-dialog')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-voice-target-line-search')),
        'GRD_263_ASGHAN',
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.textContaining('GRD_263_ASGHAN_OPEN_INFO_06_02').last,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
      await tester.pumpAndSettle();

      expect(managed.voiceTargetPublishCalls, 1);
      expect(targetGameRoot, gameRoot.path);
      expect(targetPlan?.lineId, revision3VoiceContentLineId);
      expect(targetPlan?.slotId, revision3VoiceContentSlotId);
      expect(targetPlan?.locale, 'de');
      expect(targetPlan?.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
      var state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);

      await tester.tap(voiceTools);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-build-voice-bundle')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-voice-build-dialog')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-voice-build-folder-name')),
        'asghan-home-bundle',
      );
      await tester.tap(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      final expectedOutput = p.join(buildParent.path, 'asghan-home-bundle');
      expect(find.text(expectedOutput), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-voice-build-submit')));
      await tester.pumpAndSettle();

      expect(managed.voiceBuildCalls, 1);
      expect(buildGameRoot, gameRoot.path);
      expect(buildOutput, expectedOutput);
      expect(
        find.byKey(const Key('revision3-voice-build-built')),
        findsOneWidget,
      );
      state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  testWidgets('managed Voice action is disabled without the safety game root', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\voice-needs-game-root'),
      projectId: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      contentIndexBuilder: (_) => revision3VoiceContentIndexFixture(),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    await coordinator.openManagedRevision3(managed.root);
    final container = _container(
      coordinator: coordinator,
      pickManaged: (_) async => null,
    );
    addTearDown(container.dispose);

    await _pumpApp(tester, container);
    await tester.pumpAndSettle();

    final voiceButton = find.byKey(const Key('managed-add-voice-take'));
    expect(tester.widget<FilledButton>(voiceButton).onPressed, isNull);
    expect(
      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('managed-voice-tools')),
          )
          .enabled,
      isFalse,
    );
    expect(
      find.text(
        'Configure the Gothic 1 Remake installation in Settings to author, resolve, or build Voice content and to create NPC and Quest drafts.',
      ),
      findsOneWidget,
    );
    expect(managed.voicePublishCalls, 0);
    expect(managed.voiceTargetPublishCalls, 0);
    expect(managed.voiceBuildCalls, 0);
  });

  testWidgets(
    'visible DataAsset registry adds and removes through exact managed checkpoints',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = revision3DataAssetNativeGoldenFixture();
      final stage = AuthoringRevision3DataAssetStageListResult.fromJson(
        fixture.listResponse(),
        expectedHead: fixture.stagedHead,
      ).stages.single;
      var stages = <AuthoringRevision3DataAssetStage>[];
      var pickerCalls = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\dataasset-authoring'),
        projectId: '07070707070707070707070707070707',
        projectRevision: 4,
        head: fixture.basisHead,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onDataAssetList: (_) => List.unmodifiable(stages),
        onDataAssetPublish: (lease, patchReceiptPath) {
          expect(patchReceiptPath, r'C:\verified\managed-fixture-receipt.json');
          lease.projectRevision = 5;
          lease.head = fixture.stagedHead;
          stages = <AuthoringRevision3DataAssetStage>[stage];
          return Revision3DataAssetStagePublication(
            projectId: lease.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
        onDataAssetRemove: (lease, targetPath) {
          expect(targetPath, stage.targetPath);
          lease.projectRevision = 6;
          lease.head = fixture.removedHead;
          stages = <AuthoringRevision3DataAssetStage>[];
          return Revision3DataAssetStageRemovalPublication(
            projectId: lease.projectId,
            projectRevision: 6,
            removed: stage,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        pickDataAssetPatchReceipt: () async {
          pickerCalls++;
          return r'C:\verified\managed-fixture-receipt.json';
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('managed-revision3-dataasset-tab')),
      );
      await tester.pumpAndSettle();
      expect(managed.dataAssetListCalls, 1);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-empty')),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('revision3-dataasset-stage-add')));
      await tester.pumpAndSettle();
      expect(pickerCalls, 1);
      expect(managed.dataAssetPublishCalls, 1);
      expect(managed.dataAssetListCalls, 2);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        5,
      );
      expect(find.text('TestAsset'), findsOneWidget);
      expect(find.text('Build / Deploy'), findsNothing);

      await tester.tap(find.text('TestAsset'));
      await tester.pumpAndSettle();
      final removeButton = find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      );
      await tester.ensureVisible(removeButton);
      await tester.pumpAndSettle();
      await tester.tap(removeButton);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dataasset-remove-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-remove-confirm')),
      );
      await tester.pumpAndSettle();

      expect(managed.dataAssetRemoveCalls, 1);
      expect(managed.dataAssetListCalls, 3);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 6);
      expect(state.head.canonicalJson, fixture.removedHead.canonicalJson);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-empty')),
        findsOneWidget,
      );
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets(
    'visible DataAsset value wizard publishes typed edit and reloads exact head',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = revision3DataAssetNativeGoldenFixture();
      final stage = AuthoringRevision3DataAssetStageListResult.fromJson(
        fixture.listResponse(),
        expectedHead: fixture.stagedHead,
      ).stages.single;
      var stages = <AuthoringRevision3DataAssetStage>[];
      DataAssetSemanticEditIntent? publishedIntent;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\dataasset-semantic-authoring'),
        projectId: stage.projectId,
        projectRevision: 4,
        head: fixture.basisHead,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onDataAssetList: (_) => List.unmodifiable(stages),
        onDataAssetSemanticPublish: (lease, intent) {
          publishedIntent = intent;
          lease.projectRevision = 5;
          lease.head = fixture.stagedHead;
          stages = <AuthoringRevision3DataAssetStage>[stage];
          return Revision3DataAssetStagePublication(
            projectId: lease.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        inspectDataAssetSemanticEdit:
            ({required uassetPath, required usmapPath, exportIndex}) async {
              expect(uassetPath, r'C:\proof\TestAsset.uasset');
              expect(usmapPath, r'C:\proof\Mappings.usmap');
              return DataAssetInspection.fromJson(
                validDataAssetInspectionResponse(),
              );
            },
        pickDataAssetSemanticUasset: () async => r'C:\proof\TestAsset.uasset',
        pickDataAssetSemanticUsmap: () async => r'C:\proof\Mappings.usmap',
        pickDataAssetExtractReceipt: () async =>
            r'C:\proof\extract-receipt.v2.json',
        inspectDataAssetExtractReceipt: (_) async =>
            DataAssetExtractReceiptSummary.fromJson(
              validDataAssetExtractReceiptSummaryResponse(
                targetPath: '/Game/TestAsset',
              ),
            ),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('managed-revision3-dataasset-tab')),
      );
      await tester.pumpAndSettle();

      final create = find.byKey(
        const Key('revision3-dataasset-semantic-create'),
      );
      expect(create, findsOneWidget);
      await tester.tap(create);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('dataasset-semantic-wizard')),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-pick-usmap')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-inspect')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('dataasset-semantic-editor')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('dataasset-semantic-pick-receipt')),
      );
      await tester.pumpAndSettle();
      expect(find.text('/Game/TestAsset'), findsOneWidget);
      await tester.tap(
        find.byKey(const Key('dataasset-semantic-confirm-target')),
      );
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('dataasset-semantic-value')),
        '2',
      );
      final editorScroll = tester.state<ScrollableState>(
        find
            .descendant(
              of: find.byKey(const Key('dataasset-semantic-editor')),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      editorScroll.position.jumpTo(editorScroll.position.maxScrollExtent);
      await tester.pump();
      await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
      await tester.pumpAndSettle();
      expect(find.text('Before: 1'), findsOneWidget);
      expect(find.text('After: 2'), findsOneWidget);
      editorScroll.position.jumpTo(editorScroll.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('dataasset-semantic-stage')), findsOneWidget);
      await tester.tap(find.byKey(const Key('dataasset-semantic-stage')));
      await tester.pumpAndSettle();

      expect(managed.dataAssetSemanticPublishCalls, 1);
      expect(managed.dataAssetListCalls, 2);
      expect(
        publishedIntent?.toNativeFields()['extract_receipt_path'],
        r'C:\proof\extract-receipt.v2.json',
      );
      expect(publishedIntent?.expectedTargetPath, '/Game/TestAsset');
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 5);
      expect(state.head.canonicalJson, fixture.stagedHead.canonicalJson);
      expect(find.byKey(const Key('dataasset-semantic-wizard')), findsNothing);
      expect(find.text('TestAsset'), findsOneWidget);
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets('failed managed menu open leaves the legacy shell current', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final legacy = _FakeLegacyLease(path: 'preserved.goremod');
    final expectedError = StateError('candidate rejected');
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      openManagedRevision3: (_) async => throw expectedError,
    );
    final container = _container(
      coordinator: coordinator,
      pickManaged: (_) async => r'C:\mods\invalid-r3',
    );
    addTearDown(container.dispose);

    final before = coordinator.state;
    await _pumpApp(tester, container);
    tester
        .widget<PopupMenuButton<String>>(find.byKey(const Key('project-menu')))
        .onSelected!('openManagedRevision3');
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(coordinator.state, same(before));
    expect(coordinator.state, isA<LegacyCurrentProjectState>());
    expect(legacy.closeCalls, 0);
    expect(find.byType(TabBar), findsOneWidget);
    expect(find.text('Build / Deploy'), findsOneWidget);
    expect(
      find.textContaining('Managed revision-3 project open failed:'),
      findsOneWidget,
    );
  });

  testWidgets(
    'dirty legacy cancel blocks managed picker and opener without displacement',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final legacy = _FakeLegacyLease(path: 'dirty-preserved.goremod');
      var pickerCalls = 0;
      var openerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async {
          openerCalls++;
          throw StateError('opener must not run');
        },
      );
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async {
          pickerCalls++;
          return r'C:\mods\must-not-be-picked';
        },
      );
      addTearDown(container.dispose);
      final before = coordinator.state;

      await _pumpApp(tester, container);
      container
          .read(dialogTopicsProvider.notifier)
          .setTopic(
            const DialogTopicDefinition(
              id: 'dirty_fixture',
              participantName: 'dirty_npc',
              topicClass: '/Script/Angelscript.DirtyFixture',
              sentinelClass: '/Script/Angelscript.DirtySentinel',
            ),
          );
      await tester.pump();

      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('openManagedRevision3');
      await tester.pump();

      expect(find.text('Discard unsaved changes?'), findsOneWidget);
      expect(pickerCalls, 0);
      expect(openerCalls, 0);
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(pickerCalls, 0);
      expect(openerCalls, 0);
      expect(coordinator.state, same(before));
      expect(legacy.closeCalls, 0);
      expect(find.byType(TabBar), findsOneWidget);
    },
  );

  testWidgets(
    'verification failure requires reopen and blocks menu and shortcut retries',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\requires-reopen'),
        projectId: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        projectRevision: 24,
        head: _head(24),
        verificationError: StateError('injected exact-head failure'),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsNothing,
      );

      await _sendControlS(tester);

      final poisoned = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(poisoned.requiresReopen, isTrue);
      expect(managed.verifyCalls, 1);
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsOneWidget,
      );
      expect(
        find.textContaining('This session now requires recovery'),
        findsOneWidget,
      );
      expect(
        find.textContaining('Last owned managed checkpoint'),
        findsOneWidget,
      );
      expect(
        find.text(
          'Exact-current managed identity and semantic project content.',
        ),
        findsNothing,
      );
      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('managed-create-quest-draft')),
            )
            .onPressed,
        isNull,
      );

      final menuFinder = find.byKey(const Key('project-menu'));
      final menu = tester.widget<PopupMenuButton<String>>(menuFinder);
      final verifyItem = menu
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .singleWhere((item) => item.key == const Key('project-save'));
      expect(verifyItem.enabled, isFalse);

      menu.onSelected!('save');
      for (var i = 0; i < 5; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      await _sendControlS(tester);

      expect(managed.verifyCalls, 1);
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'successful managed transition surfaces cleanup warning without details',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const privatePath = r'C:\private\retired-project.goremod';
      final legacy = _FakeLegacyLease(
        path: privatePath,
        closeError: StateError('cleanup failed at $privatePath'),
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\cleanup-warning'),
        projectId: 'cccccccccccccccccccccccccccccccc',
        projectRevision: 25,
        head: _head(25),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => managed,
      );
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => managed.root.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('openManagedRevision3');
      for (var i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }

      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(
        find.textContaining(
          'the previous project session could not be cleaned up completely',
        ),
        findsOneWidget,
      );
      expect(find.textContaining(privatePath), findsNothing);
      expect(find.textContaining('cleanup failed at'), findsNothing);
      expect(
        find.textContaining('Opened managed revision-3 project'),
        findsNothing,
      );
    },
  );
}

ProviderContainer _container({
  required CurrentProjectCoordinator coordinator,
  required ManagedRevision3DirectoryPicker pickManaged,
  String? gamePath,
  Revision3NpcCatalogLoader? loadNpcCatalog,
  Revision3NpcArchetypeChooser? chooseNpcArchetype,
  Revision3DataAssetPatchReceiptPicker? pickDataAssetPatchReceipt,
  DataAssetInspector? inspectDataAssetSemanticEdit,
  DataAssetFilePicker? pickDataAssetSemanticUasset,
  DataAssetFilePicker? pickDataAssetSemanticUsmap,
  DataAssetExtractReceiptPicker? pickDataAssetExtractReceipt,
  DataAssetExtractReceiptInspector? inspectDataAssetExtractReceipt,
  Revision3QuestCatalogLoader? loadQuestCatalog,
}) => ProviderContainer(
  overrides: [
    sharedConfigProvider.overrideWithValue(_testSharedConfig(gamePath)),
    coreServiceProvider.overrideWithValue(
      FakeGoreCoreFfiService(
        responses: const {
          'loc_status': {'ok': true, 'present': true},
          'find_game': {'ok': true, 'found': false},
        },
      ),
    ),
    currentProjectCoordinatorProvider.overrideWith((ref) => coordinator),
    managedRevision3DirectoryPickerProvider.overrideWithValue(pickManaged),
    if (loadNpcCatalog != null)
      revision3NpcCatalogLoaderProvider.overrideWithValue(loadNpcCatalog),
    if (chooseNpcArchetype != null)
      managedRevision3NpcArchetypeChooserProvider.overrideWithValue(
        chooseNpcArchetype,
      ),
    if (pickDataAssetPatchReceipt != null)
      managedRevision3DataAssetPatchReceiptPickerProvider.overrideWithValue(
        pickDataAssetPatchReceipt,
      ),
    if (inspectDataAssetSemanticEdit != null)
      managedRevision3DataAssetSemanticInspectorProvider.overrideWithValue(
        inspectDataAssetSemanticEdit,
      ),
    if (pickDataAssetSemanticUasset != null)
      managedRevision3DataAssetSemanticUassetPickerProvider.overrideWithValue(
        pickDataAssetSemanticUasset,
      ),
    if (pickDataAssetSemanticUsmap != null)
      managedRevision3DataAssetSemanticUsmapPickerProvider.overrideWithValue(
        pickDataAssetSemanticUsmap,
      ),
    if (pickDataAssetExtractReceipt != null)
      managedRevision3DataAssetExtractReceiptPickerProvider.overrideWithValue(
        pickDataAssetExtractReceipt,
      ),
    if (inspectDataAssetExtractReceipt != null)
      managedRevision3DataAssetExtractReceiptInspectorProvider
          .overrideWithValue(inspectDataAssetExtractReceipt),
    if (loadQuestCatalog != null)
      revision3QuestCatalogLoaderProvider.overrideWithValue(loadQuestCatalog),
  ],
);

SharedConfig _testSharedConfig(String? gamePath) {
  final directory = Directory.systemTemp.createTempSync('gore_home_r3_config');
  addTearDown(() {
    if (directory.existsSync()) directory.deleteSync(recursive: true);
  });
  final config = SharedConfig(File(p.join(directory.path, 'config.json')));
  if (gamePath != null) config.setGamePath(gamePath);
  return config;
}

Future<void> _setDesktopTestSurface(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1600, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

Future<void> _pumpApp(WidgetTester tester, ProviderContainer container) async {
  await tester.pumpWidget(
    UncontrolledProviderScope(container: container, child: const GoreModApp()),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}

Future<void> _sendControlS(WidgetTester tester) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  await tester.sendKeyEvent(LogicalKeyboardKey.keyS);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
  await tester.pump(const Duration(milliseconds: 300));
}

final class _FakeLegacyLease implements LegacyCurrentProjectLease {
  _FakeLegacyLease({this.path, this.closeError});

  String? path;
  final Object? closeError;
  int closeCalls = 0;

  @override
  String? get currentPath => path;

  @override
  bool get hasUnsavedChanges => false;

  @override
  Future<void> close() async {
    closeCalls++;
    final error = closeError;
    if (error != null) throw error;
  }

  @override
  Future<void> newProject() async => path = null;

  @override
  Future<void> openFromPath(String path) async => this.path = path;

  @override
  Future<void> saveCurrent() async {}

  @override
  Future<void> saveToPath(String path) async => this.path = path;
}

final class _FakeManagedLease implements ManagedRevision3CurrentProjectLease {
  _FakeManagedLease({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    this.verificationError,
    this.onNpcPublish,
    this.onQuestPublish,
    this.onQuestOutlinePublish,
    this.onVoicePublish,
    this.onVoiceTargetPublish,
    this.onVoiceBuild,
    this.onDataAssetList,
    this.onDataAssetPublish,
    this.onDataAssetSemanticPublish,
    this.onDataAssetRemove,
    this.contentIndexBuilder,
  });

  @override
  final Directory root;
  @override
  final String projectId;
  @override
  int projectRevision;
  @override
  AuthoringWorkingHead head;
  final Object? verificationError;
  final Revision3NpcDraftPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3NpcDraftAuthoringInput input,
  )?
  onNpcPublish;
  final Revision3QuestDraftPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3QuestDraftAuthoringInput input,
  )?
  onQuestPublish;
  final Revision3QuestOutlineEditPublication Function(
    _FakeManagedLease lease,
    Revision3QuestOutlineEditInput input,
  )?
  onQuestOutlinePublish;
  final Revision3VoiceTakePublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3VoiceTakeTechnicalPlan plan,
  )?
  onVoicePublish;
  final Revision3VoiceTargetPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3VoiceTargetTechnicalPlan plan,
  )?
  onVoiceTargetPublish;
  final AuthoringRevision3VoiceBuildResult Function(
    _FakeManagedLease lease,
    String gameRoot,
    String output,
  )?
  onVoiceBuild;
  final List<AuthoringRevision3DataAssetStage> Function(
    _FakeManagedLease lease,
  )?
  onDataAssetList;
  final Revision3DataAssetStagePublication Function(
    _FakeManagedLease lease,
    String patchReceiptPath,
  )?
  onDataAssetPublish;
  final Revision3DataAssetStagePublication Function(
    _FakeManagedLease lease,
    DataAssetSemanticEditIntent intent,
  )?
  onDataAssetSemanticPublish;
  final Revision3DataAssetStageRemovalPublication Function(
    _FakeManagedLease lease,
    String targetPath,
  )?
  onDataAssetRemove;
  final Revision3ContentIndex Function(_FakeManagedLease lease)?
  contentIndexBuilder;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int npcPublishCalls = 0;
  int questPublishCalls = 0;
  int questOutlinePublishCalls = 0;
  int voicePublishCalls = 0;
  int voiceTargetPublishCalls = 0;
  int voiceBuildCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetPublishCalls = 0;
  int dataAssetSemanticPublishCalls = 0;
  int dataAssetRemoveCalls = 0;
  int closeCalls = 0;

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  Future<void> close() async => closeCalls++;

  @override
  Future<Revision3ContentIndex> readContentIndex() async {
    contentReadCalls++;
    return contentIndexBuilder?.call(this) ??
        (throw StateError('fake managed lease has no content index'));
  }

  @override
  Future<Revision3QuestDraftPublication> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  }) async {
    questPublishCalls++;
    final publish = onQuestPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3QuestOutlineEditPublication>
  prepareAndPublishQuestOutlineEditV1({
    required Revision3QuestOutlineEditInput input,
  }) async {
    questOutlinePublishCalls++;
    final publish = onQuestOutlinePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest outline publisher');
    }
    return publish(this, input);
  }

  @override
  Future<Revision3NpcDraftPublication> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  }) async {
    npcPublishCalls++;
    final publish = onNpcPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no NPC publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3VoiceTakePublication> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  }) async {
    voicePublishCalls++;
    final publish = onVoicePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<Revision3VoiceTargetPublication> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  }) async {
    voiceTargetPublishCalls++;
    final publish = onVoiceTargetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice target publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  }) async {
    voiceBuildCalls++;
    final build = onVoiceBuild;
    if (build == null) {
      throw StateError('fake managed lease has no Voice builder');
    }
    return build(this, gameRoot, output);
  }

  @override
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1() async {
    dataAssetListCalls++;
    return onDataAssetList?.call(this) ?? const [];
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetStageV1({
    required String patchReceiptPath,
  }) async {
    dataAssetPublishCalls++;
    final publish = onDataAssetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no DataAsset publisher');
    }
    return publish(this, patchReceiptPath);
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  }) async {
    dataAssetSemanticPublishCalls++;
    final publish = onDataAssetSemanticPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no semantic DataAsset publisher',
      );
    }
    return publish(this, intent);
  }

  @override
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath}) async {
    dataAssetRemoveCalls++;
    final remove = onDataAssetRemove;
    if (remove == null) {
      throw StateError('fake managed lease has no DataAsset remover');
    }
    return remove(this, targetPath);
  }

  @override
  Future<void> verifyCurrentHead() async {
    verifyCalls++;
    final error = verificationError;
    if (error != null) {
      requiresReopenValue = true;
      throw error;
    }
  }
}

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

Revision3ContentIndex _contentIndex({
  required String projectId,
  required int revision,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Home Quest project',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
});

Revision3QuestCatalog _questCatalog() => Revision3QuestCatalog(
  parents: [
    Revision3QuestCatalogChoice(
      catalogId: 'parent-one',
      displayName: 'Chapter One',
    ),
  ],
  givers: [
    Revision3QuestCatalogChoice(
      catalogId: 'giver-asghan',
      displayName: 'Asghan',
    ),
  ],
);

Revision3NpcCatalog _npcCatalog() => Revision3NpcCatalog(
  choices: [
    Revision3NpcCatalogChoice(
      catalogId: 'g1r:npc:om_grd_asghan_263',
      displayName: 'Asghan guard',
    ),
  ],
);

AuthoringRevision3VoiceBuildResult _builtVoiceResult({
  required _FakeManagedLease lease,
  required String output,
}) => AuthoringRevision3VoiceBuildResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'built',
    'basis_head_json': lease.head.canonicalJson,
    'project_id': lease.projectId,
    'project_revision': lease.projectRevision,
    'output': output,
    'edit_count': 1,
    'file_count': 3,
    'bundle_bytes': 4096,
    'bundle_sha256':
        'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    'build_authority': 'generation_sealed_existing_member_bundle_v1',
    'deployment_status': 'not_performed',
  },
  expectedHead: lease.head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    projectId: lease.projectId,
    projectRevision: lease.projectRevision,
  ),
  expectedOutput: output,
);
