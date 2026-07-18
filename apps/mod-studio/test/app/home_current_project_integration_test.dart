import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/shared_config.dart';
import 'package:gore_mod/app/domain/ui_settings.dart'
    show localeProvider, sharedConfigProvider;
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/dataasset/ui/dataasset_lab.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';
import 'package:gore_mod/gore_mod_app.dart';
import 'package:gore_mod/home_page.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
import 'package:gore_mod/project/revision3_project_history.dart';
import 'package:gore_mod/project/revision3_project_import.dart';
import 'package:gore_mod/project/revision3_base_game_content_browser.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_creation_authoring.dart';
import 'package:gore_mod/project/revision3_global_content_search.dart';
import 'package:gore_mod/project/revision3_managed_compiler_check_panel.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_greeting_authoring.dart';
import 'package:gore_mod/project/revision3_npc_wizard.dart';
import 'package:gore_mod/project/revision3_project_command_bar.dart';
import 'package:gore_mod/project/revision3_project_problems.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';
import 'package:gore_mod/project/revision3_quest_journey_view.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';
import 'package:gore_mod/project/revision3_localization_voice_workspace.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_build_dialog.dart';
import 'package:gore_mod/project/revision3_voice_folder_authoring.dart';
import 'package:gore_mod/project/revision3_voice_production_card.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_dialog.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_status_authoring.dart';
import 'package:gore_mod/project/revision3_voice_target_dialog.dart';
import 'package:gore_mod/project/revision3_voice_wizard.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_npc_fixture.dart';
import '../support/revision3_project_problems_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_preview_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

const _homeQuestTranscriptQuestId = '77777777777777777777777777777777';
const _homeQuestTranscriptModuleId = '88888888888888888888888888888888';
const _homeNpcGreetingNpcId = '79797979797979797979797979797979';
const _homeNpcGreetingModuleId = '89898989898989898989898989898989';
const _homeQuestOpeningRecipeQuestId = '67676767676767676767676767676767';
const _homeQuestOpeningRecipeModuleId = '68686868686868686868686868686868';
const _homeQuestOpeningRecipeTechnicalId = 'GORE_QUEST_OPENING_RECIPE';
const _homeCreatedNpcId = '29292929292929292929292929292929';
const _homeCreatedNpcModuleId = '39393939393939393939393939393939';
const _homeCreatedNpcUniqueName = 'GORE_NORTH_GATE_GUARD';
const _homeCreatedNpcModuleNamespace = 'GoreMods.Npcs.NorthGateGuard';

void main() {
  test(
    'Voice folder success receipt is silent after a project switch',
    () async {
      final oldRoot = Directory(r'C:\mods\voice-folder-toast-old');
      final newRoot = Directory(r'C:\mods\voice-folder-toast-new');
      const oldProjectId = '81818181818181818181818181818181';
      final oldProject = _FakeManagedLease(
        root: oldRoot,
        projectId: oldProjectId,
        projectRevision: 8,
        head: _head(8),
      );
      final newProject = _FakeManagedLease(
        root: newRoot,
        projectId: '82828282828282828282828282828282',
        projectRevision: 3,
        head: _head(3),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async =>
            root.path == oldRoot.path ? oldProject : newProject,
      );
      await coordinator.openManagedRevision3(oldRoot);
      final publication = Revision3VoiceFolderImportPublication(
        projectId: oldProjectId,
        projectRevision: 8,
        projectHead: _head(8).canonicalJson,
        checkpointToken: 'checkpoint-8',
        planToken: 'plan-8',
        importedCount: 1,
      );

      expect(
        revision3VoiceFolderPublicationMatchesCurrent(
          coordinator.state,
          originRoot: oldRoot.path,
          originProjectId: oldProjectId,
          publication: publication,
        ),
        isTrue,
      );

      await coordinator.closeCurrent();
      await coordinator.openManagedRevision3(newRoot);
      expect(
        revision3VoiceFolderPublicationMatchesCurrent(
          coordinator.state,
          originRoot: oldRoot.path,
          originProjectId: oldProjectId,
          publication: publication,
        ),
        isFalse,
      );
    },
  );

  testWidgets('visible Legacy entry creates and adopts a managed R3 project', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final gameRoot = Directory.systemTemp.createTempSync('gore_r3_create_game');
    Directory(p.join(gameRoot.path, 'G1R')).createSync();
    final destination = Directory.systemTemp.createTempSync(
      'gore_r3_create_project',
    );
    addTearDown(() {
      if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      if (destination.existsSync()) destination.deleteSync(recursive: true);
    });
    final legacy = _FakeLegacyLease(path: 'before-create.goremod');
    final managed = _FakeManagedLease(
      root: destination,
      projectId: 'edededededededededededededededed',
      projectRevision: 0,
      head: _head(0),
    );
    ManagedRevision3ProjectCreateRequest? received;
    var pickerCalls = 0;
    String? pickerLabel;
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      createManagedRevision3: (request) async {
        received = request;
        return managed;
      },
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    final container = _container(
      coordinator: coordinator,
      gamePath: gameRoot.path,
      pickManaged: (label) async {
        pickerCalls++;
        pickerLabel = label;
        return destination.path;
      },
    );
    addTearDown(container.dispose);

    await _pumpApp(tester, container);
    expect(
      find.byKey(const Key('legacy-compatibility-banner')),
      findsOneWidget,
    );
    final compatibilityCopy = tester.widget<Text>(
      find.byKey(const Key('legacy-compatibility-tools-description')),
    );
    expect(
      compatibilityCopy.textSpan?.toPlainText(),
      contains('Legacy compatibility tools'),
    );
    expect(
      compatibilityCopy.textSpan?.toPlainText(),
      contains('older direct-replacement tools'),
    );
    expect(
      find.byKey(const Key('managed-project-entry-create')),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const Key('managed-project-entry-create')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(
      find.byKey(const Key('revision3-project-create-dialog')),
      findsOneWidget,
    );
    expect(pickerCalls, 0);
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-name')),
      'Asghan Expanded',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-author')),
      'Gore Team',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-locales')),
      'de, en-US',
    );
    await tester.tap(find.byKey(const Key('revision3-project-create-submit')));
    for (var index = 0; index < 10; index++) {
      await tester.pump(const Duration(milliseconds: 50));
    }

    expect(pickerCalls, 1);
    expect(pickerLabel, 'Create managed mod project here');
    expect(received?.root.path, destination.path);
    expect(received?.gameRoot, gameRoot.path);
    expect(received?.name, 'Asghan Expanded');
    expect(received?.version, '0.1.0');
    expect(received?.author, 'Gore Team');
    expect(received?.authoringLocales, const <String>['de', 'en-US']);
    expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
    expect(legacy.closeCalls, 1);
    expect(
      find.byKey(const Key('managed-revision3-project-view')),
      findsOneWidget,
    );
    expect(find.textContaining('Created managed mod project'), findsOneWidget);
  });

  testWidgets(
    'NPC starter creates empty revision zero before guided revision one',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_starter_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      final destination = Directory.systemTemp.createTempSync(
        'gore_r3_npc_starter_project',
      );
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
        if (destination.existsSync()) destination.deleteSync(recursive: true);
      });

      final managed = _FakeManagedLease(
        root: destination,
        projectId: 'cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd',
        projectRevision: 0,
        head: _head(0),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(lease.projectRevision, 0);
          expect(lease.head.canonicalJson, _head(0).canonicalJson);
          expect(requestedGameRoot, gameRoot.path);
          expect(input.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
          expect(input.displayName, 'Starter Guard');
          lease.projectRevision = 1;
          lease.head = _head(1);
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: 1,
            head: lease.head,
            npcId: '10101010101010101010101010101010',
            scriptModuleId: '20202020202020202020202020202020',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        createManagedRevision3: (_) async => managed,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );
      final container = _container(
        coordinator: coordinator,
        gamePath: gameRoot.path,
        pickManaged: (_) async => destination.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.tap(find.byKey(const Key('managed-project-entry-create')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-starter-npc-draft')),
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-name')),
        'Guard starter',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-author')),
        'Gore Team',
      );
      final create = find.byKey(const Key('revision3-project-create-submit'));
      await tester.ensureVisible(create);
      await tester.tap(create);
      await tester.pumpAndSettle();

      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        0,
      );
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);

      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Starter Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      expect(managed.npcPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        1,
      );
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(
        find.textContaining('NPC starter saved in project revision 1'),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'uncertain NPC starter publication requires reopen instead of claiming empty',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_uncertain_npc_starter_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      final destination = Directory.systemTemp.createTempSync(
        'gore_r3_uncertain_npc_starter_project',
      );
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
        if (destination.existsSync()) destination.deleteSync(recursive: true);
      });

      final managed = _FakeManagedLease(
        root: destination,
        projectId: 'dededededededededededededededede',
        projectRevision: 0,
        head: _head(0),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          lease.requiresReopenValue = true;
          throw StateError('fixture publication outcome is uncertain');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        createManagedRevision3: (_) async => managed,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );
      final container = _container(
        coordinator: coordinator,
        gamePath: gameRoot.path,
        pickManaged: (_) async => destination.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.tap(find.byKey(const Key('managed-project-entry-create')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-starter-npc-draft')),
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-name')),
        'Uncertain NPC starter',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-author')),
        'Gore Team',
      );
      final create = find.byKey(const Key('revision3-project-create-submit'));
      await tester.ensureVisible(create);
      await tester.tap(create);
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Uncertain Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      final poisoned = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(poisoned.requiresReopen, isTrue);
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);

      await tester.tap(find.byKey(const Key('revision3-npc-cancel')));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(
        find.textContaining('cannot verify the starter outcome'),
        findsOneWidget,
      );
      expect(
        find.textContaining('valid empty project remains current'),
        findsNothing,
      );
    },
  );

  testWidgets('cancelled Quest starter keeps the valid empty project', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final gameRoot = Directory.systemTemp.createTempSync(
      'gore_r3_quest_starter_game',
    );
    Directory(p.join(gameRoot.path, 'G1R')).createSync();
    final destination = Directory.systemTemp.createTempSync(
      'gore_r3_quest_starter_project',
    );
    addTearDown(() {
      if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      if (destination.existsSync()) destination.deleteSync(recursive: true);
    });

    final managed = _FakeManagedLease(
      root: destination,
      projectId: 'efefefefefefefefefefefefefefefef',
      projectRevision: 0,
      head: _head(0),
    );
    final coordinator = CurrentProjectCoordinator(
      createManagedRevision3: (_) async => managed,
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    final container = _container(
      coordinator: coordinator,
      gamePath: gameRoot.path,
      pickManaged: (_) async => destination.path,
      loadQuestCatalog: (_) async => _questCatalog(),
    );
    addTearDown(container.dispose);

    await _pumpApp(tester, container);
    await tester.tap(find.byKey(const Key('managed-project-entry-create')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-project-starter-quest-draft')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-name')),
      'Quest starter',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-author')),
      'Gore Team',
    );
    final create = find.byKey(const Key('revision3-project-create-submit'));
    await tester.ensureVisible(create);
    await tester.tap(create);
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-quest-wizard')), findsOneWidget);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState)
          .projectRevision,
      0,
    );
    await tester.tap(find.byKey(const Key('revision3-quest-cancel')));
    await tester.pumpAndSettle();

    expect(managed.questPublishCalls, 0);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState)
          .projectRevision,
      0,
    );
    expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
    expect(
      find.textContaining('valid empty project remains current'),
      findsOneWidget,
    );
  });

  testWidgets(
    'compact Legacy shell keeps managed entries and all tabs reachable',
    (tester) async {
      await _setNarrowShortTestSurface(tester);
      final legacy = _FakeLegacyLease(path: 'compact-legacy.goremod');
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pump(const Duration(milliseconds: 100));

      expect(tester.takeException(), isNull);
      expect(
        find.byKey(const Key('legacy-build-deploy-compact')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('legacy-compatibility-banner-scroll')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('legacy-compatibility-banner')),
        findsOneWidget,
      );
      final tabBar = tester.widget<TabBar>(find.byType(TabBar));
      expect(tabBar.tabs, hasLength(8));
      expect(find.byType(TabBarView), findsOneWidget);

      for (final key in const [
        Key('managed-project-entry-create'),
        Key('managed-project-entry-open'),
        Key('managed-project-entry-restore'),
      ]) {
        final entry = find.byKey(key);
        expect(entry, findsOneWidget);
        await tester.ensureVisible(entry);
        await tester.pump();
        expect(entry.hitTestable(), findsOneWidget);
        expect(tester.takeException(), isNull);
      }

      expect(find.byType(TabBar), findsOneWidget);
      expect(find.byType(TabBarView), findsOneWidget);
    },
  );

  testWidgets('new managed project requires a configured game root first', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    var creatorCalls = 0;
    var pickerCalls = 0;
    final coordinator = CurrentProjectCoordinator(
      createManagedRevision3: (_) async {
        creatorCalls++;
        throw StateError('creator must not run');
      },
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    final container = _container(
      coordinator: coordinator,
      pickManaged: (_) async {
        pickerCalls++;
        return null;
      },
    );
    addTearDown(container.dispose);

    await _pumpApp(tester, container);
    expect(find.byKey(const Key('managed-project-landing')), findsOneWidget);
    expect(find.byKey(const Key('legacy-compatibility-banner')), findsNothing);
    expect(
      find.byKey(const Key('managed-project-entry-settings')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('managed-project-entry-create')),
      findsOneWidget,
    );
    tester.view.physicalSize = const Size(640, 420);
    await tester.pumpAndSettle();
    expect(
      tester.takeException(),
      isNull,
      reason: 'the managed-project landing remains usable when narrow/short',
    );
    final createEntry = find.byKey(const Key('managed-project-entry-create'));
    await tester.ensureVisible(createEntry);
    await tester.pumpAndSettle();
    await tester.tap(createEntry);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(
      find.byKey(const Key('revision3-project-create-dialog')),
      findsNothing,
    );
    expect(pickerCalls, 0);
    expect(creatorCalls, 0);
    expect(
      find.textContaining('Set the Gothic 1 Remake game path'),
      findsOneWidget,
    );
    tester.view.physicalSize = const Size(1600, 900);
    await tester.pumpAndSettle();

    final settingsEntry = find.byKey(
      const Key('managed-project-entry-settings'),
    );
    await tester.ensureVisible(settingsEntry);
    await tester.pumpAndSettle();
    await tester.tap(settingsEntry);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('managed-settings-dialog')), findsOneWidget);
    expect(pickerCalls, 0);
    expect(creatorCalls, 0);

    await tester.tap(find.byKey(const Key('managed-settings-close')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('managed-settings-dialog')), findsNothing);
    expect(pickerCalls, 0);
    expect(creatorCalls, 0);
  });

  testWidgets(
    'managed open owns the shell, Ctrl+S verifies, and Close releases it',
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
      final legacyMenuFinder = find.byKey(const Key('project-menu'));
      expect(
        tester
            .widget<PopupMenuButton<String>>(legacyMenuFinder)
            .itemBuilder(tester.element(legacyMenuFinder))
            .whereType<PopupMenuItem<String>>()
            .where(
              (item) =>
                  item.key == const Key('project-export-managed-revision3'),
            ),
        isEmpty,
      );
      expect(
        find.byKey(const Key('legacy-compatibility-banner')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('managed-project-entry-open')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('managed-project-entry-open')));
      for (var i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(requestedRoot?.path, managed.root.path);
      expect(
        find.byKey(const Key('managed-revision3-project-view')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('legacy-compatibility-banner')),
        findsNothing,
      );
      await _expandManagedTechnicalDetails(tester);
      expect(find.text(managed.root.path), findsOneWidget);
      expect(find.text(managed.projectId), findsOneWidget);
      expect(find.text('${managed.projectRevision}'), findsOneWidget);
      expect(find.text(managed.head.snapshotSha256), findsOneWidget);
      expect(find.text('Build / Deploy'), findsNothing);
      expect(
        find.byKey(const Key('revision3-project-workspace-tabbar')),
        findsOneWidget,
      );
      expect(find.byType(NavigationRail), findsNothing);
      for (final key in _managedPrimaryNavigationKeys) {
        expect(find.byKey(key), findsOneWidget);
      }
      await _navigateManagedDataAssets(tester);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-panel')),
        findsOneWidget,
      );
      expect(find.textContaining('Support is checked'), findsOneWidget);
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
      final close = menu
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .singleWhere((item) => item.key == const Key('project-close'));
      expect(close.enabled, isTrue);
      expect((close.child as Text).data, 'Close project');
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

      await tester.tap(menuFinder);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('project-close')));
      await tester.pumpAndSettle();
      expect(managed.closeCalls, 1);
      expect(
        find.byKey(const Key('managed-revision3-project-view')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'managed project copy is available from the Project menu with one global busy lane',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync('gore_home_export_');
      addTearDown(() => parent.deleteSync(recursive: true));
      final completion =
          Completer<AuthoringRevision3ExactSnapshotExportResultV2>();
      late String pendingOutput;
      final managed = _FakeExportManagedLease(
        root: Directory(r'C:\mods\managed-project-export'),
        projectId: 'abababababababababababababababab',
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onExport: (lease, output) {
          pendingOutput = output;
          return completion.future;
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      var pickerLabel = '';
      final container = _container(
        coordinator: coordinator,
        pickManaged: (label) async {
          pickerLabel = label;
          return parent.path;
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('managed-export-project-copy')),
        findsNothing,
        reason: 'project export belongs to the Project menu, not Home',
      );

      final menuFinder = find.byKey(const Key('project-menu'));
      List<PopupMenuItem<String>> menuItems() => tester
          .widget<PopupMenuButton<String>>(menuFinder)
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .toList();
      PopupMenuItem<String> exportItem() => menuItems().singleWhere(
        (item) => item.key == const Key('project-export-managed-revision3'),
      );
      expect(exportItem().enabled, isTrue);
      final managedActionOrder = menuItems()
          .where(
            (item) => {
              Key('project-save'),
              Key('project-export-managed-revision3'),
              Key('project-close'),
            }.contains(item.key),
          )
          .map((item) => item.key)
          .toList();
      expect(managedActionOrder, const <Key>[
        Key('project-save'),
        Key('project-export-managed-revision3'),
        Key('project-close'),
      ]);

      await tester.tap(menuFinder);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('project-export-managed-revision3')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-export-dialog')),
        findsOneWidget,
      );
      expect(exportItem().enabled, isFalse);

      await tester.tap(
        find.byKey(const Key('revision3-project-export-choose-parent')),
      );
      await tester.pumpAndSettle();
      expect(pickerLabel, 'Choose destination folder');
      await tester.tap(
        find.byKey(const Key('revision3-project-export-submit')),
      );
      await tester.pump();
      expect(managed.exportCalls, 1);
      expect(
        find.byKey(const Key('revision3-project-export-progress')),
        findsOneWidget,
      );

      completion.complete(
        _homeProjectExportResult(
          head: managed.head,
          projectId: managed.projectId,
          projectRevision: managed.projectRevision,
          output: pendingOutput,
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-export-published')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('revision3-project-export-close')));
      await tester.pumpAndSettle();
      expect(exportItem().enabled, isTrue);

      await tester.tap(menuFinder);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('project-export-managed-revision3')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-export-dialog')),
        findsOneWidget,
      );
      managed.projectRevision = 8;
      managed.head = _head(8);
      await tester.tap(
        find.byKey(const Key('revision3-project-export-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-export-submit')),
      );
      await tester.pumpAndSettle();
      expect(managed.exportCalls, 1);
      expect(find.textContaining('No output was created'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-project-export-submit')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'project backup restore adopts only the exact receipt and preserves both cleanup warnings',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync('gore_home_restore_');
      addTearDown(() {
        if (parent.existsSync()) parent.deleteSync(recursive: true);
      });
      final source = p.join(parent.path, 'asghan-backup.goremod');
      File(source).writeAsBytesSync(const <int>[1]);
      const projectId = 'cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd';
      const projectRevision = 7;
      final head = _head(projectRevision);
      final destination = p.join(
        parent.path,
        'restored-project-r$projectRevision',
      );
      final candidate = _FakeManagedLease(
        root: Directory(destination),
        projectId: projectId,
        projectRevision: projectRevision,
        head: head,
      );
      final legacy = _FakeLegacyLease(
        path: 'legacy-before-restore.goremod',
        closeError: StateError(r'cleanup failed at C:\private\legacy.lock'),
      );
      var openerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (root) async {
          openerCalls++;
          expect(root.path, destination);
          return candidate;
        },
      );
      var sourcePickerCalls = 0;
      var inspectorCalls = 0;
      var importerCalls = 0;
      String? parentPickerLabel;
      Revision3ProjectImportDestinationRequest? receivedRequest;
      final container = _container(
        coordinator: coordinator,
        pickManaged: (label) async {
          parentPickerLabel = label;
          return parent.path;
        },
        pickProjectBackup: () async {
          sourcePickerCalls++;
          return source;
        },
        inspectProjectBackup: (selectedSource) async {
          inspectorCalls++;
          expect(selectedSource, source);
          return _homeProjectImportInspectionResponse(
            source: source,
            projectId: projectId,
            projectRevision: projectRevision,
            head: head,
          );
        },
        restoreProjectBackup: (request) async {
          importerCalls++;
          receivedRequest = request;
          return _homeProjectImportDestinationResponse(
            request: request,
            projectId: projectId,
            projectRevision: projectRevision,
            head: head,
            outcome: 'imported_with_cleanup_warning',
          );
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      expect(
        find.byKey(const Key('managed-project-entry-restore')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('managed-project-entry-restore')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.byKey(const Key('revision3-project-import-dialog')),
        findsOneWidget,
      );
      final l10n = AppLocalizations.of(
        tester.element(
          find.byKey(const Key('revision3-project-import-dialog')),
        ),
      );

      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-source')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('asghan-backup.goremod'), findsOneWidget);
      expect(find.text(source), findsNothing);
      expect(find.text('8192'), findsOneWidget);

      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-parent')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(parentPickerLabel, l10n.projectRestoreChooseDestinationParent);
      expect(find.text(destination), findsOneWidget);

      await tester.ensureVisible(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      for (var index = 0; index < 5; index++) {
        await tester.pump(const Duration(milliseconds: 100));
      }

      expect(sourcePickerCalls, 1);
      expect(inspectorCalls, 1);
      expect(importerCalls, 1);
      expect(openerCalls, 1);
      expect(receivedRequest?.source, source);
      expect(receivedRequest?.destination, destination);
      expect(receivedRequest?.expectedArchive.byteLength, 8192);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.root.path, destination);
      expect(current.projectId, projectId);
      expect(current.projectRevision, projectRevision);
      expect(current.head.canonicalJson, head.canonicalJson);
      expect(legacy.closeCalls, 1);
      expect(candidate.closeCalls, 0);
      expect(
        find.text(l10n.projectRestoreOpenedCleanupWarning),
        findsOneWidget,
      );
      expect(find.textContaining(r'C:\private'), findsNothing);

      await tester.pump(const Duration(seconds: 5));
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text(l10n.projectTransitionCleanupWarning), findsOneWidget);
    },
  );

  testWidgets(
    'cleanup-warning restore reports safe open, native cleanup, and candidate cleanup failures',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync(
        'gore_home_restore_open_failure_',
      );
      addTearDown(() {
        if (parent.existsSync()) parent.deleteSync(recursive: true);
      });
      final source = p.join(parent.path, 'failed-open-backup.goremod');
      File(source).writeAsBytesSync(const <int>[1]);
      const projectId = 'abababababababababababababababab';
      const projectRevision = 9;
      final head = _head(projectRevision);
      final destination = p.join(
        parent.path,
        'restored-project-r$projectRevision',
      );
      const candidateCleanupPrivatePath = r'C:\private\restored-candidate.lock';
      final mismatchedCandidate = _FakeManagedLease(
        root: Directory(destination),
        projectId: 'bcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbc',
        projectRevision: projectRevision,
        head: head,
        closeError: StateError(
          'candidate cleanup failed at $candidateCleanupPrivatePath',
        ),
      );
      final legacy = _FakeLegacyLease(path: 'legacy-stays-current.goremod');
      var openerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (root) async {
          openerCalls++;
          expect(root.path, destination);
          return mismatchedCandidate;
        },
      );
      var importerCalls = 0;
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => parent.path,
        pickProjectBackup: () async => source,
        inspectProjectBackup: (_) async => _homeProjectImportInspectionResponse(
          source: source,
          projectId: projectId,
          projectRevision: projectRevision,
          head: head,
        ),
        restoreProjectBackup: (request) async {
          importerCalls++;
          return _homeProjectImportDestinationResponse(
            request: request,
            projectId: projectId,
            projectRevision: projectRevision,
            head: head,
            outcome: 'imported_with_cleanup_warning',
          );
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.tap(find.byKey(const Key('managed-project-entry-restore')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-source')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-parent')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.ensureVisible(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      final l10n = AppLocalizations.of(
        tester.element(
          find.byKey(const Key('revision3-project-import-submit')),
        ),
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      for (var index = 0; index < 8 && openerCalls == 0; index++) {
        await tester.pump(const Duration(milliseconds: 100));
      }
      await tester.pump(const Duration(milliseconds: 300));

      expect(importerCalls, 1);
      expect(openerCalls, 1);
      expect(mismatchedCandidate.closeCalls, 1);
      expect(legacy.closeCalls, 0);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(
        find.text(
          l10n.projectRestoreOpenFailed('restored-project-r$projectRevision'),
        ),
        findsOneWidget,
      );
      expect(find.textContaining(parent.path), findsNothing);
      expect(find.textContaining('bcbcbcbcbcbcbcbc'), findsNothing);
      expect(find.textContaining(candidateCleanupPrivatePath), findsNothing);

      final cleanupWarning = find.text(
        l10n.projectRestoreSucceededCleanupWarning,
      );
      for (
        var index = 0;
        index < 8 && cleanupWarning.evaluate().isEmpty;
        index++
      ) {
        await tester.pump(const Duration(seconds: 1));
      }
      expect(cleanupWarning, findsOneWidget);
      expect(find.textContaining(parent.path), findsNothing);
      expect(find.textContaining(candidateCleanupPrivatePath), findsNothing);

      final candidateCleanupWarning = find.text(
        l10n.projectRestoreCandidateCleanupWarning,
      );
      for (
        var index = 0;
        index < 8 && candidateCleanupWarning.evaluate().isEmpty;
        index++
      ) {
        await tester.pump(const Duration(seconds: 1));
      }
      expect(candidateCleanupWarning, findsOneWidget);
      expect(find.textContaining(candidateCleanupPrivatePath), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'receipt adoption keeps non-dismissible opening progress and project actions blocked',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync(
        'gore_home_restore_open_progress_',
      );
      addTearDown(() {
        if (parent.existsSync()) parent.deleteSync(recursive: true);
      });
      final source = p.join(parent.path, 'delayed-open-backup.goremod');
      File(source).writeAsBytesSync(const <int>[1]);
      const projectId = 'cacacacacacacacacacacacacacacaca';
      const projectRevision = 6;
      final head = _head(projectRevision);
      final destination = p.join(
        parent.path,
        'restored-project-r$projectRevision',
      );
      final candidate = _FakeManagedLease(
        root: Directory(destination),
        projectId: projectId,
        projectRevision: projectRevision,
        head: head,
      );
      final opener = Completer<ManagedRevision3CurrentProjectLease>();
      var openerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) {
          openerCalls++;
          expect(root.path, destination);
          return opener.future;
        },
      );
      var sourcePickerCalls = 0;
      var importerCalls = 0;
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => parent.path,
        pickProjectBackup: () async {
          sourcePickerCalls++;
          return source;
        },
        inspectProjectBackup: (_) async => _homeProjectImportInspectionResponse(
          source: source,
          projectId: projectId,
          projectRevision: projectRevision,
          head: head,
        ),
        restoreProjectBackup: (request) async {
          importerCalls++;
          return _homeProjectImportDestinationResponse(
            request: request,
            projectId: projectId,
            projectRevision: projectRevision,
            head: head,
            outcome: 'imported',
          );
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.tap(find.byKey(const Key('managed-project-entry-restore')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-source')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-parent')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.ensureVisible(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      for (var index = 0; index < 5 && openerCalls == 0; index++) {
        await tester.pump(const Duration(milliseconds: 100));
      }

      expect(importerCalls, 1);
      expect(openerCalls, 1);
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsOneWidget,
      );
      expect(
        tester
            .widgetList<ModalBarrier>(find.byType(ModalBarrier))
            .any((barrier) => !barrier.dismissible),
        isTrue,
      );
      for (final key in const <Key>[
        Key('managed-project-entry-create'),
        Key('managed-project-entry-open'),
        Key('managed-project-entry-restore'),
        Key('managed-project-entry-settings'),
      ]) {
        final button = tester.widget<ButtonStyleButton>(find.byKey(key));
        expect(button.onPressed, isNull, reason: '$key must stay disabled');
      }
      final projectMenu = tester.widget<PopupMenuButton<String>>(
        find.byKey(const Key('project-menu')),
      );
      final restoreItem = projectMenu
          .itemBuilder(tester.element(find.byKey(const Key('project-menu'))))
          .whereType<PopupMenuItem<String>>()
          .singleWhere(
            (item) =>
                item.key == const Key('project-restore-managed-revision3'),
          );
      expect(restoreItem.enabled, isFalse);

      projectMenu.onSelected!('restoreManagedRevision3');
      await tester.pump();
      expect(sourcePickerCalls, 1);
      expect(importerCalls, 1);
      expect(openerCalls, 1);
      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsOneWidget,
      );
      expect(coordinator.state, isA<NoCurrentProjectState>());

      opener.complete(candidate);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsNothing,
      );
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(candidate.closeCalls, 0);
      expect(
        find.text(
          AppLocalizations.of(
            tester.element(find.byType(Scaffold)),
          ).projectRestoreOpened,
        ),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'uncertain project backup publication never opens or retries a project',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync(
        'gore_home_restore_uncertain_',
      );
      addTearDown(() {
        if (parent.existsSync()) parent.deleteSync(recursive: true);
      });
      final source = p.join(parent.path, 'uncertain-backup.goremod');
      File(source).writeAsBytesSync(const <int>[1]);
      const projectId = 'dededededededededededededededede';
      const projectRevision = 4;
      final head = _head(projectRevision);
      var openerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async {
          openerCalls++;
          throw StateError('must not open an uncertain destination');
        },
      );
      var sourcePickerCalls = 0;
      var importerCalls = 0;
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => parent.path,
        pickProjectBackup: () async {
          sourcePickerCalls++;
          return source;
        },
        inspectProjectBackup: (_) async => _homeProjectImportInspectionResponse(
          source: source,
          projectId: projectId,
          projectRevision: projectRevision,
          head: head,
        ),
        restoreProjectBackup: (request) async {
          importerCalls++;
          return _homeProjectImportDestinationResponse(
            request: request,
            projectId: projectId,
            projectRevision: projectRevision,
            head: head,
            outcome: 'publication_uncertain',
          );
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.tap(find.byKey(const Key('managed-project-entry-restore')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-source')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-import-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.ensureVisible(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pumpAndSettle();

      expect(importerCalls, 1);
      expect(openerCalls, 0);
      expect(coordinator.state, isA<NoCurrentProjectState>());
      expect(
        find.textContaining('Studio cannot prove whether the project folder'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-import-submit')),
        findsNothing,
      );
      final sourceButton = tester.widget<OutlinedButton>(
        find.byKey(const Key('revision3-project-import-choose-source')),
      );
      expect(sourceButton.onPressed, isNull);
      expect(sourcePickerCalls, 1);

      await tester.tap(find.byKey(const Key('revision3-project-import-close')));
      await tester.pumpAndSettle();
      expect(openerCalls, 0);
      expect(importerCalls, 1);
      expect(coordinator.state, isA<NoCurrentProjectState>());
    },
  );

  testWidgets(
    'dirty managed project text guards Open, Restore, and Close without losing the draft',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const projectId = '91919191919191919191919191919191';
      const localizationId = '92929292929292929292929292929292';
      const localizationRevision = 4;
      const locId = 'GORE_ASGHAN_DIRTY_GUARD';
      const draft = 'Ungespeicherter Asghan-Text';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\managed-localization-dirty'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _dialogLocalizationEditIndex(
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
          localizationId: localizationId,
          localizationRevision: localizationRevision,
          locId: locId,
        ),
        onDialogLocalizationEditSeed:
            (lease, requestedId, requestedRevision, requestedLocId) {
              expect(requestedId, localizationId);
              expect(requestedRevision, localizationRevision);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: requestedId,
                localizationRevision: requestedRevision,
                locId: requestedLocId,
              );
            },
        onDialogLocalizationEditPublish: (_, _) =>
            throw StateError('dirty-guard test must not publish the draft'),
      );
      var pickerCalls = 0;
      var backupPickerCalls = 0;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        gamePath: r'C:\Games\G1R\Gothic1Remake.exe',
        pickManaged: (_) async {
          pickerCalls++;
          return r'C:\mods\replacement-managed-project';
        },
        pickProjectBackup: () async {
          backupPickerCalls++;
          return r'C:\backups\must-not-open.goremod';
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);
      expect(managed.dialogLocalizationEditSeedCalls, 1);
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-import-voice-folder'),
        enabled: true,
      );

      final textField = find.byKey(const Key('revision3-localization-text-de'));
      expect(textField, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(textField));
      await tester.enterText(textField, draft);
      await tester.pump();
      await tester.pump();

      final undo = find.byKey(Revision3ProjectCommandBar.undoKey);
      expect(undo, findsOneWidget);
      expect(tester.widget<OutlinedButton>(undo).onPressed, isNull);
      expect(
        find.byTooltip(l10n.managedProjectHistoryDirtyBlocked),
        findsOneWidget,
      );

      final projectMenu = find.byKey(const Key('project-menu'));
      final exportItem = tester
          .widget<PopupMenuButton<String>>(projectMenu)
          .itemBuilder(tester.element(projectMenu))
          .whereType<PopupMenuItem<String>>()
          .singleWhere(
            (item) => item.key == const Key('project-export-managed-revision3'),
          );
      expect(exportItem.enabled, isFalse);

      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('openManagedRevision3');
      await tester.pumpAndSettle();

      expect(find.text(l10n.managedLocalizationUnsavedTitle), findsOneWidget);
      expect(pickerCalls, 0);
      expect(managed.closeCalls, 0);
      await tester.tap(
        find.widgetWithText(TextButton, l10n.managedLocalizationKeepEditing),
      );
      await tester.pumpAndSettle();

      expect(pickerCalls, 0);
      expect(managed.closeCalls, 0);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(tester.widget<TextField>(textField).controller!.text, draft);

      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('restoreManagedRevision3');
      await tester.pumpAndSettle();

      expect(find.text(l10n.managedLocalizationUnsavedTitle), findsOneWidget);
      expect(backupPickerCalls, 0);
      expect(managed.closeCalls, 0);
      await tester.tap(
        find.widgetWithText(TextButton, l10n.managedLocalizationKeepEditing),
      );
      await tester.pumpAndSettle();

      expect(backupPickerCalls, 0);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      expect(tester.widget<TextField>(textField).controller!.text, draft);

      tester
          .widget<PopupMenuButton<String>>(
            find.byKey(const Key('project-menu')),
          )
          .onSelected!('close');
      await tester.pumpAndSettle();

      expect(find.text(l10n.managedLocalizationUnsavedTitle), findsOneWidget);
      expect(managed.closeCalls, 0);
      await tester.tap(
        find.widgetWithText(FilledButton, l10n.managedLocalizationDiscard),
      );
      await tester.pumpAndSettle();

      expect(managed.closeCalls, 1);
      expect(managed.dialogLocalizationEditPublishCalls, 0);
      expect(coordinator.state, isA<NoCurrentProjectState>());
      expect(
        find.byKey(const Key('managed-revision3-project-view')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'dirty localization gates Story creation and every Story mutation surface',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
      const draft = 'Unsaved authority-gate text';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-dirty-authority-gate'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _questTranscriptHomeIndex(revision: lease.projectRevision),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance question',
                speaker: 'Asghan',
                voiceSlotLocales: const <String>{'de'},
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
        gamePath: r'C:\Games\G1R\Gothic1Remake.exe',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);

      final textField = find.byKey(const Key('revision3-localization-text-en'));
      expect(textField, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(textField));
      final blockedReason = l10n.managedStoryWorkspaceMutationDirtyBlocked;
      await tester.enterText(textField, draft);
      await tester.pump();
      await tester.pump();

      await _navigateManagedStory(tester);
      final createOpening = find.byKey(
        const Key('revision3-story-workspace-create-quest-opening'),
      );
      final createNpcOpening = find.byKey(
        const Key('revision3-story-workspace-create-npc-opening'),
      );
      expect(tester.widget<FilledButton>(createOpening).onPressed, isNull);
      expect(tester.widget<FilledButton>(createNpcOpening).onPressed, isNull);
      final createReason = find.byKey(
        const Key(
          'revision3-story-workspace-create-npc-opening-disabled-reason',
        ),
      );
      expect(createReason, findsOneWidget);
      expect(tester.widget<Text>(createReason).data, blockedReason);

      final advancedCreate = find.byKey(
        const Key('revision3-story-workspace-create-advanced'),
      );
      await tester.ensureVisible(advancedCreate);
      await tester.pumpAndSettle();
      await tester.tap(advancedCreate);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .enabled,
        isFalse,
      );
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-quest')),
            )
            .enabled,
        isFalse,
      );
      await tester.tapAt(const Offset(2, 2));
      await tester.pumpAndSettle();

      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$_homeQuestTranscriptQuestId',
        ),
      );
      expect(selectedQuest, findsOneWidget);
      expect(tester.widget<ListTile>(selectedQuest).selected, isTrue);
      final journeyView = tester.widget<Revision3QuestJourneyView>(
        find.byType(Revision3QuestJourneyView),
      );
      expect(journeyView.onEditNameObjectives, isNull);
      expect(journeyView.onEditDescriptionConnections, isNull);
      expect(journeyView.onEditStatesTransitions, isNull);
      for (final key in const <Key>[
        Key('revision3-quest-journey-edit-name-objectives'),
        Key('revision3-quest-journey-edit-description-connections'),
        Key('revision3-quest-journey-edit-states-transitions'),
      ]) {
        final action = find.byKey(key);
        expect(action, findsOneWidget);
        expect(tester.widget<ButtonStyleButton>(action).onPressed, isNull);
      }
      expect(find.text(blockedReason), findsWidgets);
      expect(
        find.byKey(
          const Key(
            'revision3-story-workbench-tab-logic-'
            '$_homeQuestTranscriptQuestId',
          ),
        ),
        findsNothing,
      );

      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, _homeQuestTranscriptQuestId);
      expect(
        find.byKey(
          const Key(
            'revision3-story-workbench-action-edit-overview-'
            '$_homeQuestTranscriptQuestId',
          ),
        ),
        findsNothing,
      );
      final reopenedJourneyEdit = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      expect(
        tester.widget<OutlinedButton>(reopenedJourneyEdit).onPressed,
        isNull,
      );
      expect(find.text(blockedReason), findsWidgets);

      expect(managed.npcPublishCalls, 0);
      expect(managed.questPublishCalls, 0);
      expect(managed.questOutlinePublishCalls, 0);
      expect(managed.questContextPublishCalls, 0);
      expect(managed.questTransitionsPublishCalls, 0);
      expect(managed.dialogLocalizationEditPublishCalls, 0);
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, 7);
      expect(current.head.canonicalJson, _head(7).canonicalJson);
      expect(
        find.byKey(const Key('revision3-quest-outline-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-quest-transitions-dialog')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'dirty listener stays lifecycle-safe across managed project replacement and teardown',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const firstProjectId = '81818181818181818181818181818181';
      const localizationId = '82828282828282828282828282828282';
      const localizationRevision = 3;
      const locId = 'GORE_DIRTY_REPLACEMENT';
      final first = _FakeManagedLease(
        root: Directory(r'C:\mods\managed-dirty-replacement-first'),
        projectId: firstProjectId,
        projectRevision: 5,
        head: _head(5),
        contentIndexBuilder: (lease) => _dialogLocalizationEditIndex(
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
          localizationId: localizationId,
          localizationRevision: localizationRevision,
          locId: locId,
        ),
        onDialogLocalizationEditSeed:
            (lease, requestedId, requestedRevision, requestedLocId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: requestedId,
                  localizationRevision: requestedRevision,
                  locId: requestedLocId,
                ),
      );
      final second = _FakeManagedLease(
        root: Directory(r'C:\mods\managed-dirty-replacement-second'),
        projectId: '83838383838383838383838383838383',
        projectRevision: 1,
        head: _head(1),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async =>
            root.path == second.root.path ? second : first,
      );
      await coordinator.openManagedRevision3(first.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => second.root.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);
      final textField = find.byKey(const Key('revision3-localization-text-de'));
      await tester.enterText(textField, 'Dirty before direct replacement');
      await tester.pump();
      await tester.pump();

      await coordinator.openManagedRevision3(second.root);
      await tester.pumpAndSettle();
      final visible = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(visible.projectId, second.projectId);
      expect(first.closeCalls, 1);
      expect(tester.takeException(), isNull);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'managed content scopes stay lazy and show setup without game evidence',
    (tester) async {
      await _setDesktopTestSurface(tester);
      var baseCatalogCalls = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\scoped-content-no-game'),
        projectId: '41414141414141414141414141414141',
        projectRevision: 4,
        head: _head(4),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        loadBaseGameCatalog: (_) async {
          baseCatalogCalls++;
          return _baseGameCatalog();
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await _navigateManagedBaseGameContent(tester);
      expect(
        find.byKey(
          const Key('revision3-base-game-content-browser-missing-game'),
        ),
        findsOneWidget,
      );
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await _navigateManagedInstalledContent(tester);
      expect(
        find.byKey(const Key('revision3-installed-content-browser-setup')),
        findsOneWidget,
      );
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await tester.tap(
        find.byKey(
          const Key('revision3-installed-content-browser-setup-action'),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-settings-expert-page')),
        findsOneWidget,
      );
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);
    },
  );

  testWidgets(
    'Search all stays lazy until submit and opens an exact This mod result',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_global_content_search_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const targetEntityId = '92929292929292929292929292929292';
      var baseCatalogCalls = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\global-content-search'),
        projectId: '43434343434343434343434343434343',
        projectRevision: 6,
        head: _head(6),
        contentIndexBuilder: (lease) => _globalSearchContentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          targetEntityId: targetEntityId,
        ),
        onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          return _homeDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
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
        loadBaseGameCatalog: (_) async {
          baseCatalogCalls++;
          return _baseGameCatalog();
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      final contentReadsBeforeOpeningContent = managed.contentReadCalls;
      expect(contentReadsBeforeOpeningContent, greaterThanOrEqualTo(1));
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await _navigateManagedContent(tester);
      expect(
        find.byKey(
          const Key('revision3-scoped-content-browser-nav-all-sources'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-scoped-content-browser-page-all-sources'),
        ),
        findsNothing,
        reason: 'the fourth source page has not been visited yet',
      );
      final contentReadsBeforeGlobalSearch = managed.contentReadCalls;
      expect(
        contentReadsBeforeGlobalSearch,
        greaterThanOrEqualTo(contentReadsBeforeOpeningContent),
      );
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-scoped-content-browser-nav-all-sources'),
      );
      expect(
        find.byKey(
          const Key('revision3-scoped-content-browser-page-all-sources'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-global-content-search-empty-prompt')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const ValueKey<Object>((
            'revision3-global-content-source',
            Revision3GlobalContentSource.thisMod,
          )),
        ),
        findsNothing,
        reason: 'mounting Search all does not begin a search implicitly',
      );
      expect(managed.contentReadCalls, contentReadsBeforeGlobalSearch);
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await tester.enterText(
        find.byKey(const Key('revision3-global-content-search-field')),
        'asghan',
      );
      await tester.tap(
        find.byKey(const Key('revision3-global-content-search-submit')),
      );
      await tester.pumpAndSettle();

      expect(managed.contentReadCalls, contentReadsBeforeGlobalSearch + 1);
      expect(baseCatalogCalls, 1);
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(find.text('Asghan Sentinel'), findsOneWidget);
      expect(find.text('Asghan guard'), findsOneWidget);
      expect(find.text('DA_Asghan'), findsOneWidget);
      for (final source in Revision3GlobalContentSource.values) {
        expect(
          find.byKey(
            ValueKey<Object>(('revision3-global-content-source', source)),
          ),
          findsOneWidget,
        );
      }

      final openTarget = find.byKey(
        const ValueKey<Object>((
          'global-search-action',
          Revision3GlobalContentActionKind.openThisModEntity,
          targetEntityId,
        )),
      );
      await tester.ensureVisible(openTarget);
      await tester.tap(openTarget);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-scoped-content-browser-page-this-mod')),
        findsOneWidget,
      );
      final targetTile = find.byKey(
        const Key('revision3-content-entity-$targetEntityId'),
      );
      expect(targetTile, findsOneWidget);
      expect(tester.widget<ListTile>(targetTile).selected, isTrue);
      expect(managed.contentReadCalls, contentReadsBeforeGlobalSearch + 1);
      expect(baseCatalogCalls, 1);
      expect(managed.dataAssetPackageIndexReadCalls, 1);
    },
  );

  testWidgets('Base game and Installed scopes expose exact bounded workflows', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final gameRoot = Directory.systemTemp.createTempSync(
      'gore_r3_scoped_content_game',
    );
    Directory(p.join(gameRoot.path, 'G1R')).createSync();
    addTearDown(() {
      if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
    });
    var baseCatalogCalls = 0;
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\scoped-content'),
      projectId: '42424242424242424242424242424242',
      projectRevision: 5,
      head: _head(5),
      contentIndexBuilder: (lease) => _contentIndex(
        projectId: lease.projectId,
        revision: lease.projectRevision,
      ),
      onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
        expect(requestedGameRoot, gameRoot.path);
        return _homeDataAssetPackageIndexResult(
          head: lease.head,
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
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
      loadBaseGameCatalog: (_) async {
        baseCatalogCalls++;
        return _baseGameCatalog();
      },
      loadNpcCatalog: (_) async => _npcCatalog(),
    );
    addTearDown(container.dispose);

    await _pumpApp(tester, container);
    await tester.pumpAndSettle();
    expect(baseCatalogCalls, 0);
    expect(managed.dataAssetPackageIndexReadCalls, 0);

    await _navigateManagedBaseGameContent(tester);
    expect(baseCatalogCalls, 1);
    expect(managed.dataAssetPackageIndexReadCalls, 0);
    expect(find.text('Asghan guard'), findsOneWidget);
    expect(find.text('Chapter One'), findsOneWidget);
    final npcStart = find.byKey(
      const ValueKey((
        'revision3-base-game-create-npc',
        'g1r:npc:om_grd_asghan_263',
      )),
    );
    await tester.ensureVisible(npcStart);
    await tester.tap(npcStart);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-npc-selected-archetype-label')),
      findsOneWidget,
    );
    expect(find.text('Asghan guard'), findsWidgets);
    await tester.tap(find.byKey(const Key('revision3-npc-cancel')));
    await tester.pumpAndSettle();

    await _navigateManagedInstalledContent(tester);
    expect(baseCatalogCalls, 1);
    expect(managed.dataAssetPackageIndexReadCalls, 1);
    await tester.enterText(
      find.byKey(const Key('revision3-installed-content-browser-search')),
      'asghan',
    );
    await tester.pump();
    expect(
      find.byKey(const ValueKey('revision3-installed-content-browser-row-0')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const ValueKey('revision3-installed-content-browser-open-0')),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsOneWidget,
    );
    final dialogSearch = tester.widget<TextField>(
      find.byKey(const Key('installed-package-browser-search')),
    );
    expect(dialogSearch.controller?.text, '/Game/Characters/DA_Asghan');
    expect(managed.dataAssetPackageIndexReadCalls, 2);
    expect(find.text('DA_Asghan'), findsWidgets);
    expect(find.text('DA_Viper'), findsNothing);
  });

  testWidgets(
    'Base game NPC starting point publishes and opens exact Story Dialog Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_base_game_npc_handoff',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = 'a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7';
      var baseCatalogLoads = 0;
      var npcCatalogLoads = 0;
      var chooserCalls = 0;
      Revision3NpcDraftPublication? published;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\base-game-npc-handoff'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => lease.projectRevision == 7
            ? _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              )
            : _createdNpcContentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
                npcId: _homeCreatedNpcId,
                moduleId: _homeCreatedNpcModuleId,
                displayName: 'North Gate Guard',
              ),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
          expect(input.displayName, 'North Gate Guard');
          lease.projectRevision = 8;
          lease.head = _head(8);
          return published = Revision3NpcDraftPublication(
            projectId: projectId,
            projectRevision: 8,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
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
        loadBaseGameCatalog: (_) async {
          baseCatalogLoads++;
          return _baseGameCatalog();
        },
        loadNpcCatalog: (_) async {
          npcCatalogLoads++;
          return _npcCatalog();
        },
        chooseNpcArchetype: (_, _) async {
          chooserCalls++;
          return 'g1r:npc:om_grd_viper_000';
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedBaseGameContent(tester);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
        reason: 'Story stays lazy until the exact Base game action publishes',
      );

      final npcStart = find.byKey(
        const ValueKey((
          'revision3-base-game-create-npc',
          'g1r:npc:om_grd_asghan_263',
        )),
      );
      await tester.ensureVisible(npcStart);
      await tester.tap(npcStart);
      await tester.pumpAndSettle();

      final wizard = find.byKey(const Key('revision3-npc-wizard'));
      expect(wizard, findsOneWidget);
      expect(
        find.byKey(const Key('revision3-npc-selected-archetype-label')),
        findsOneWidget,
      );
      expect(find.text('Asghan guard'), findsWidgets);
      expect(
        find.descendant(
          of: wizard,
          matching: find.text('g1r:npc:om_grd_asghan_263'),
        ),
        findsNothing,
      );
      expect(chooserCalls, 0, reason: 'the exact Base game row is preselected');
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'North Gate Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      expect(baseCatalogLoads, 1);
      expect(npcCatalogLoads, 2);
      expect(chooserCalls, 0);
      expect(managed.npcPublishCalls, 1);
      expect(managed.contentReadCalls, greaterThanOrEqualTo(3));
      expect(published?.projectId, projectId);
      expect(published?.projectRevision, 8);
      expect(published?.head.canonicalJson, _head(8).canonicalJson);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      _expectExactCreatedNpcStoryDialogVoice(tester);
    },
  );

  testWidgets(
    'Global Search Base NPC publishes and opens exact Story Dialog Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_global_search_npc_handoff',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = 'b8b8b8b8b8b8b8b8b8b8b8b8b8b8b8b8';
      var baseCatalogLoads = 0;
      var npcCatalogLoads = 0;
      var chooserCalls = 0;
      Revision3NpcDraftPublication? published;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\global-search-npc-handoff'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => lease.projectRevision == 7
            ? _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              )
            : _createdNpcContentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
                npcId: _homeCreatedNpcId,
                moduleId: _homeCreatedNpcModuleId,
                displayName: 'North Gate Guard',
              ),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
          expect(input.displayName, 'North Gate Guard');
          lease.projectRevision = 8;
          lease.head = _head(8);
          return published = Revision3NpcDraftPublication(
            projectId: projectId,
            projectRevision: 8,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
          );
        },
        onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          return _homeDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
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
        loadBaseGameCatalog: (_) async {
          baseCatalogLoads++;
          return _baseGameCatalog();
        },
        loadNpcCatalog: (_) async {
          npcCatalogLoads++;
          return _npcCatalog();
        },
        chooseNpcArchetype: (_, _) async {
          chooserCalls++;
          return 'g1r:npc:om_grd_viper_000';
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedContent(tester);
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-scoped-content-browser-nav-all-sources'),
      );
      await tester.enterText(
        find.byKey(const Key('revision3-global-content-search-field')),
        'asghan',
      );
      await tester.tap(
        find.byKey(const Key('revision3-global-content-search-submit')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
        reason: 'Story stays lazy until the exact Search all action publishes',
      );
      expect(baseCatalogLoads, 1);
      expect(managed.dataAssetPackageIndexReadCalls, 1);

      final createNpc = find.byKey(
        const ValueKey<Object>((
          'global-search-action',
          Revision3GlobalContentActionKind.createBaseNpcDraft,
          'g1r:npc:om_grd_asghan_263',
        )),
      );
      await tester.ensureVisible(createNpc);
      await tester.tap(createNpc);
      await tester.pumpAndSettle();

      final wizard = find.byKey(const Key('revision3-npc-wizard'));
      expect(wizard, findsOneWidget);
      expect(
        find.byKey(const Key('revision3-npc-selected-archetype-label')),
        findsOneWidget,
      );
      expect(find.text('Asghan guard'), findsWidgets);
      expect(
        find.descendant(
          of: wizard,
          matching: find.text('g1r:npc:om_grd_asghan_263'),
        ),
        findsNothing,
      );
      expect(
        chooserCalls,
        0,
        reason: 'the exact Search all result is preselected',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'North Gate Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      expect(baseCatalogLoads, 1);
      expect(npcCatalogLoads, 2);
      expect(chooserCalls, 0);
      expect(managed.npcPublishCalls, 1);
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(managed.contentReadCalls, greaterThanOrEqualTo(4));
      expect(published?.projectId, projectId);
      expect(published?.projectRevision, 8);
      expect(published?.head.canonicalJson, _head(8).canonicalJson);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      _expectExactCreatedNpcStoryDialogVoice(tester);
    },
  );

  testWidgets(
    'canonical managed workspace hosts real tools with honest availability',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final currentHead = _head(3);
      const projectId = 'cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd';
      final managed = _FakeHistoryManagedLease(
        root: Directory(r'C:\mods\canonical-workspace'),
        projectId: projectId,
        projectRevision: 3,
        head: currentHead,
        history: Revision3ProjectHistorySnapshot(
          basisHead: currentHead,
          projectId: projectId,
          currentRevision: 3,
          entries: <Revision3ProjectHistoryEntry>[
            Revision3ProjectHistoryEntry(
              head: currentHead,
              projectId: projectId,
              projectRevision: 3,
              isCurrent: true,
            ),
            Revision3ProjectHistoryEntry(
              head: _head(2),
              projectId: projectId,
              projectRevision: 2,
              isCurrent: false,
            ),
          ],
          historyTruncated: false,
        ),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
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

      for (final key in _managedPrimaryNavigationKeys) {
        expect(find.byKey(key), findsOneWidget);
      }
      expect(
        find.byKey(const Key('revision3-project-dashboard')),
        findsOneWidget,
      );
      for (final key in const <Key>[
        Key('managed-home-story'),
        Key('managed-home-dialog-voice'),
        Key('managed-home-problems'),
        Key('managed-home-content'),
        Key('managed-home-build'),
      ]) {
        expect(find.byKey(key), findsOneWidget);
        expect(tester.widget<ListTile>(find.byKey(key)).onTap, isNotNull);
      }
      expect(
        find.descendant(
          of: find.byKey(const Key('revision3-project-dashboard-tasks')),
          matching: find.byType(ListTile),
        ),
        findsNWidgets(5),
      );
      expect(find.byKey(const Key('managed-review-problems')), findsNothing);

      await _navigateManagedDataAssets(tester);
      expect(managed.dataAssetListCalls, 1);
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      expect(
        find.byKey(const Key('revision3-story-workspace')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-section-story-page')),
        findsNothing,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                const Key('revision3-story-workspace-create-npc-opening'),
              ),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                const Key('revision3-story-workspace-create-quest-opening'),
              ),
            )
            .onPressed,
        isNull,
      );
      final advancedCreate = find.byKey(
        const Key('revision3-story-workspace-create-advanced'),
      );
      expect(advancedCreate, findsOneWidget);
      await tester.ensureVisible(advancedCreate);
      await tester.pumpAndSettle();
      await tester.tap(advancedCreate);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .enabled,
        isFalse,
      );
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-quest')),
            )
            .enabled,
        isFalse,
      );
      await tester.tapAt(const Offset(2, 2));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-empty')),
        findsOneWidget,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-content'),
      );
      expect(
        find.byKey(const Key('revision3-content-workspace-page-data-assets')),
        findsOneWidget,
        reason: 'Content remembers its last secondary route',
      );
      expect(managed.dataAssetListCalls, 1);

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      expect(
        find.byKey(const Key('revision3-story-workspace')),
        findsOneWidget,
        reason: 'Story is a direct workspace, not a Content launcher',
      );
      expect(
        find.byKey(const Key('revision3-content-workspace-page-library')),
        findsNothing,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-world'),
      );
      expect(
        find.byKey(const Key('revision3-project-section-world-page')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-project-section-world-status-world-authoring'),
        ),
        findsOneWidget,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-localization-voice'),
      );
      expect(
        find.byKey(const Key('revision3-localization-voice-workspace')),
        findsOneWidget,
      );
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-manage-voice'),
        enabled: false,
      );
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-resolve-voice'),
        enabled: false,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );
      expect(
        find.byKey(const Key('revision3-project-problems-view')),
        findsOneWidget,
      );
      final verifyAssessment = find.byKey(
        const Key('revision3-project-problems-verify-current-project'),
      );
      expect(verifyAssessment, findsOneWidget);
      await tester.ensureVisible(verifyAssessment);
      await tester.tap(verifyAssessment);
      await tester.pump(const Duration(milliseconds: 300));
      expect(managed.verifyCalls, 1);

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-build-release'),
      );
      expect(
        find.byKey(const Key('revision3-project-section-build-release-page')),
        findsOneWidget,
      );
      _expectManagedSectionAction(
        tester,
        sectionId: 'build-release',
        actionId: 'build-voice-bundle',
        enabled: false,
      );
      _expectManagedSectionAction(
        tester,
        sectionId: 'build-release',
        actionId: 'build-playable-mod',
        enabled: false,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-history'),
      );
      expect(
        find.byKey(const Key('revision3-project-history-page')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-history-entry-3')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-history-entry-2')),
        findsOneWidget,
      );
      expect(managed.historyReadCalls, 1);

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-settings-expert'),
      );
      expect(
        find.byKey(const Key('revision3-settings-expert-page')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-settings-expert-page-settings')),
        findsOneWidget,
      );

      await _navigateManagedHome(tester);
      expect(
        find.byKey(const Key('revision3-project-dashboard')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'global Undo from Story loads fresh History, confirms friendly, and rebinds N plus 1',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const currentRevision = 7;
      const restoredFromRevision = 6;
      const nextRevision = 8;
      final currentProjectJson = revision3VoiceFixtureProjectJson(
        revision: currentRevision,
      );
      final currentHead = revision3DataAssetHeadForProject(currentProjectJson);
      final restoredFromHead = revision3DataAssetHeadForProject(
        revision3VoiceFixtureProjectJson(revision: restoredFromRevision),
      );
      final history = Revision3ProjectHistorySnapshot(
        basisHead: currentHead,
        projectId: revision3VoiceFixtureProjectId,
        currentRevision: currentRevision,
        entries: <Revision3ProjectHistoryEntry>[
          Revision3ProjectHistoryEntry(
            head: currentHead,
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: currentRevision,
            isCurrent: true,
          ),
          Revision3ProjectHistoryEntry(
            head: restoredFromHead,
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: restoredFromRevision,
            isCurrent: false,
          ),
        ],
        historyTruncated: false,
      );
      late final _FakeHistoryManagedLease managed;
      managed = _FakeHistoryManagedLease(
        root: Directory(r'C:\mods\global-undo-story'),
        projectId: revision3VoiceFixtureProjectId,
        projectRevision: currentRevision,
        head: currentHead,
        canonicalProjectJsonValue: currentProjectJson,
        history: history,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onRestore: (lease, expectedHistory, target) {
          expect(identical(expectedHistory, history), isTrue);
          expect(target.projectRevision, restoredFromRevision);
          expect(target.head.canonicalJson, restoredFromHead.canonicalJson);
          final nextProjectJson = revision3VoiceFixtureProjectJson(
            revision: nextRevision,
          );
          final nextHead = revision3DataAssetHeadForProject(nextProjectJson);
          lease
            ..projectRevision = nextRevision
            ..head = nextHead
            ..canonicalProjectJsonValue = nextProjectJson;
          return ManagedRevision3ProjectHistoryRestoreCheckpoint(
            previousHead: currentHead,
            head: nextHead,
            projectJson: nextProjectJson,
            projectId: revision3VoiceFixtureProjectId,
            previousProjectRevision: currentRevision,
            projectRevision: nextRevision,
            restoredFromHead: target.head,
            restoredFromRevision: target.projectRevision,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      expect(managed.historyReadCalls, 0);
      expect(managed.historyRestoreCalls, 0);
      final undo = find.byKey(Revision3ProjectCommandBar.undoKey);
      expect(undo, findsOneWidget);
      expect(tester.widget<OutlinedButton>(undo).onPressed, isNotNull);
      final contentReadsBeforeRestore = managed.contentReadCalls;

      await tester.tap(undo);
      await tester.pumpAndSettle();

      expect(managed.historyReadCalls, 1);
      expect(managed.historyRestoreCalls, 0);
      expect(
        find.byKey(const Key('revision3-project-global-undo-dialog')),
        findsOneWidget,
      );
      expect(
        find.text(
          'The content from revision 6 will be saved as new revision 8. '
          'The current version remains in history.',
        ),
        findsOneWidget,
      );
      expect(
        find.text(
          'Only the project changes. The game installation and save files '
          'remain untouched.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining(revision3VoiceFixtureProjectId), findsNothing);
      expect(find.textContaining(managed.root.path), findsNothing);
      expect(
        find.textContaining(currentHead.snapshotSha256.substring(0, 12)),
        findsNothing,
      );

      await tester.tap(
        find.byKey(const Key('revision3-project-global-undo-confirm')),
      );
      await tester.pumpAndSettle();

      expect(managed.historyReadCalls, 1);
      expect(managed.historyRestoreCalls, 1);
      expect(managed.projectRevision, nextRevision);
      final visible = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(visible.projectRevision, nextRevision);
      expect(visible.head.canonicalJson, managed.head.canonicalJson);
      expect(managed.contentReadCalls, greaterThan(contentReadsBeforeRestore));
      expect(
        find.byKey(const Key('revision3-story-workspace')),
        findsOneWidget,
      );
      expect(
        find.text('Revision 6 was restored as a new project version.'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'global Undo from Voice needs no game root and Cancel publishes nothing',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const currentRevision = 5;
      final currentProjectJson = revision3VoiceFixtureProjectJson(
        revision: currentRevision,
      );
      final currentHead = revision3DataAssetHeadForProject(currentProjectJson);
      final history = Revision3ProjectHistorySnapshot(
        basisHead: currentHead,
        projectId: revision3VoiceFixtureProjectId,
        currentRevision: currentRevision,
        entries: <Revision3ProjectHistoryEntry>[
          Revision3ProjectHistoryEntry(
            head: currentHead,
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: currentRevision,
            isCurrent: true,
          ),
          Revision3ProjectHistoryEntry(
            head: revision3DataAssetHeadForProject(
              revision3VoiceFixtureProjectJson(revision: 4),
            ),
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: 4,
            isCurrent: false,
          ),
        ],
        historyTruncated: false,
      );
      final managed = _FakeHistoryManagedLease(
        root: Directory(r'C:\mods\global-undo-voice'),
        projectId: revision3VoiceFixtureProjectId,
        projectRevision: currentRevision,
        head: currentHead,
        canonicalProjectJsonValue: currentProjectJson,
        history: history,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-localization-voice'),
      );
      expect(managed.historyReadCalls, 0);
      final undo = find.byKey(Revision3ProjectCommandBar.undoKey);
      expect(undo, findsOneWidget);
      expect(tester.widget<OutlinedButton>(undo).onPressed, isNotNull);

      await tester.tap(undo);
      await tester.pumpAndSettle();
      expect(managed.historyReadCalls, 1);
      expect(
        find.byKey(const Key('revision3-project-global-undo-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-global-undo-cancel')),
      );
      await tester.pumpAndSettle();

      expect(managed.historyRestoreCalls, 0);
      expect(managed.projectRevision, currentRevision);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        currentRevision,
      );
      expect(
        find.textContaining('was restored as a new project version'),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('global Undo reports a freshly proven empty History', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    const revision = 3;
    final projectJson = revision3VoiceFixtureProjectJson(revision: revision);
    final head = revision3DataAssetHeadForProject(projectJson);
    final managed = _FakeHistoryManagedLease(
      root: Directory(r'C:\mods\global-undo-empty'),
      projectId: revision3VoiceFixtureProjectId,
      projectRevision: revision,
      head: head,
      canonicalProjectJsonValue: projectJson,
      history: Revision3ProjectHistorySnapshot(
        basisHead: head,
        projectId: revision3VoiceFixtureProjectId,
        currentRevision: revision,
        entries: <Revision3ProjectHistoryEntry>[
          Revision3ProjectHistoryEntry(
            head: head,
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: revision,
            isCurrent: true,
          ),
        ],
        historyTruncated: false,
      ),
      contentIndexBuilder: (lease) => _contentIndex(
        projectId: lease.projectId,
        revision: lease.projectRevision,
      ),
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
    expect(managed.historyReadCalls, 0);
    await tester.tap(find.byKey(Revision3ProjectCommandBar.undoKey));
    await tester.pumpAndSettle();

    expect(managed.historyReadCalls, 1);
    expect(managed.historyRestoreCalls, 0);
    expect(
      find.text('No previous project versions have been recorded yet.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-global-undo-dialog')),
      findsNothing,
    );
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState)
          .projectRevision,
      revision,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'direct Story reports an exact-selection miss after Quest publish',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_story_workspace_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-workspace'),
        projectId: '56565656565656565656565656565656',
        projectRevision: 3,
        head: _head(3),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onQuestPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          lease.projectRevision = 4;
          lease.head = _head(4);
          return Revision3QuestDraftPublication(
            projectId: lease.projectId,
            projectRevision: 4,
            questId: '57575757575757575757575757575757',
            scriptModuleId: '58585858585858585858585858585858',
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
        loadNpcCatalog: (_) async => _npcCatalog(),
        loadQuestCatalog: (_) async => _questCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );

      final createNpcOpening = find.byKey(
        const Key('revision3-story-workspace-create-npc-opening'),
      );
      expect(
        tester.widget<FilledButton>(createNpcOpening).onPressed,
        isNotNull,
      );
      final advancedCreate = find.byKey(
        const Key('revision3-story-workspace-create-advanced'),
      );
      await tester.ensureVisible(advancedCreate);
      await tester.tap(advancedCreate);
      await tester.pumpAndSettle();
      final createNpcDraft = find.byKey(
        const Key('revision3-story-workspace-create-npc'),
      );
      expect(
        tester.widget<PopupMenuItem<dynamic>>(createNpcDraft).enabled,
        isTrue,
      );
      await tester.tap(createNpcDraft);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-npc-cancel')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);

      await _openAdvancedQuestCreation(tester);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byKey(const Key('revision3-quest-wizard')), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-quest-title')),
        'Story route Quest',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-description')),
        'Prove the direct Story creation route.',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-objective')),
        'Return to Story',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
      expect(managed.questPublishCalls, 1);
      expect(managed.projectRevision, 4);
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.textContaining(
          'could not be selected at its exact project revision',
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-story-workspace')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'direct Story Quest transcript hands the exact line to Localization and Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-quest-transcript'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _questTranscriptHomeIndex(revision: lease.projectRevision),
        onDialogLocalizationRead:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationReadResult(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                nonemptyPreview: 'Homer ist noch nicht zurueck.',
              );
            },
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance question',
                speaker: 'Asghan',
                voiceSlotLocales: const <String>{'de'},
                germanText: 'Homer ist noch nicht zurueck.',
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );

      await tester.tap(
        find.byKey(
          const Key(
            'revision3-story-workspace-entity-$_homeQuestTranscriptQuestId',
          ),
        ),
      );
      await tester.pumpAndSettle();
      final dialogVoiceTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$_homeQuestTranscriptQuestId',
        ),
      );
      await tester.ensureVisible(dialogVoiceTab);
      await tester.tap(dialogVoiceTab);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-quest-transcript-panel')),
        findsOneWidget,
      );
      expect(find.text('Homer ist noch nicht zurueck.'), findsOneWidget);
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      final openTextVoice = find.byKey(
        const Key('revision3-quest-transcript-open-text-voice'),
      );
      final outerTranscriptScroll = find
          .ancestor(of: openTextVoice, matching: find.byType(Scrollable))
          .last;
      await tester.scrollUntilVisible(
        openTextVoice,
        240,
        scrollable: outerTranscriptScroll,
      );
      await tester.ensureVisible(openTextVoice);
      await tester.pumpAndSettle();
      expect(tester.widget<FilledButton>(openTextVoice).onPressed, isNotNull);

      await tester.tap(openTextVoice);
      await tester.pumpAndSettle();

      expect(find.byType(Revision3LocalizationVoiceWorkspace), findsOneWidget);
      expect(managed.dialogLocalizationEditSeedCalls, greaterThanOrEqualTo(1));
      final germanText = find.byKey(
        const Key('revision3-localization-text-de'),
      );
      expect(germanText, findsOneWidget);
      expect(
        tester.widget<TextField>(germanText).controller?.text,
        'Homer ist noch nicht zurueck.',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'direct Story NPC greetings hand the exact line to Localization and Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-npc-greetings'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _npcGreetingHomeIndex(revision: lease.projectRevision),
        onDialogLocalizationRead:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationReadResult(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                nonemptyPreview: 'Willkommen am Mineneingang.',
              );
            },
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance greeting',
                speaker: 'Asghan',
                voiceSlotLocales: const <String>{'de'},
                germanText: 'Willkommen am Mineneingang.',
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );

      await tester.tap(
        find.byKey(
          const Key('revision3-story-workspace-entity-$_homeNpcGreetingNpcId'),
        ),
      );
      await tester.pumpAndSettle();
      final dialogVoiceTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$_homeNpcGreetingNpcId',
        ),
      );
      await tester.ensureVisible(dialogVoiceTab);
      await tester.tap(dialogVoiceTab);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-npc-dialog-voice-panel')),
        findsOneWidget,
      );
      expect(find.text('Willkommen am Mineneingang.'), findsOneWidget);
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      expect(find.text(_homeNpcGreetingNpcId), findsNothing);
      final openTextVoice = find.byKey(
        const Key('revision3-npc-greeting-open-text-voice'),
      );
      await tester.ensureVisible(openTextVoice);
      await tester.pumpAndSettle();
      expect(tester.widget<FilledButton>(openTextVoice).onPressed, isNotNull);

      await tester.tap(openTextVoice);
      await tester.pumpAndSettle();

      expect(find.byType(Revision3LocalizationVoiceWorkspace), findsOneWidget);
      expect(managed.dialogLocalizationEditSeedCalls, greaterThanOrEqualTo(1));
      final germanText = find.byKey(
        const Key('revision3-localization-text-de'),
      );
      expect(germanText, findsOneWidget);
      expect(
        tester.widget<TextField>(germanText).controller?.text,
        'Willkommen am Mineneingang.',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Story NPC no-slot greeting plans exact Voice intent and rebinds its work item',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_story_npc_voice_plan_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
      Revision3DialogVoiceSlotCreationTechnicalPlan? publishedPlan;
      String? plannedSlotId;
      final managed = _FakeDialogVoiceSlotCreationManagedLease(
        root: Directory(r'C:\mods\story-npc-voice-plan'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _npcGreetingHomeIndex(
          revision: lease.projectRevision,
          existingDeSlot: lease.projectRevision == 8,
          lineRevision: lease.projectRevision == 8 ? 3 : 2,
          slotRevision: 0,
          slotId: plannedSlotId ?? revision3VoiceContentSlotId,
        ),
        onDialogLocalizationRead:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationReadResult(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                nonemptyPreview: 'Willkommen am Mineneingang.',
              );
            },
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) {
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(localizationRevision, 0);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: requestedLocId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance greeting',
                speaker: 'Asghan',
                voiceSlotLocales: lease.projectRevision == 8
                    ? const <String>{'de'}
                    : const <String>{},
                germanText: 'Willkommen am Mineneingang.',
              );
            },
        onDialogVoiceSlotCreation: (lease, plan) {
          expect(lease.projectRevision, 7);
          expect(plan.lineId, revision3VoiceContentLineId);
          expect(plan.expectedLineRevision, 2);
          expect(plan.localizationId, revision3VoiceContentLocalizationId);
          expect(plan.expectedLocalizationRevision, 0);
          expect(plan.locId, locId);
          expect(plan.locale, 'de');
          expect(plan.slotId, matches(RegExp(r'^[0-9a-f]{32}$')));
          publishedPlan = plan;
          plannedSlotId = plan.slotId;
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3DialogVoiceSlotCreationPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            lineId: plan.lineId,
            lineRevision: plan.expectedLineRevision + 1,
            localizationId: plan.localizationId,
            localizationRevision: plan.expectedLocalizationRevision,
            slotId: plan.slotId,
            slotRevision: 0,
            locale: plan.locale,
            locId: plan.locId,
            targetResolution: Revision3ContentVoiceTargetResolution.unresolved,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      await tester.tap(
        find.byKey(
          const Key('revision3-story-workspace-entity-$_homeNpcGreetingNpcId'),
        ),
      );
      await tester.pumpAndSettle();
      final dialogVoiceTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$_homeNpcGreetingNpcId',
        ),
      );
      await tester.ensureVisible(dialogVoiceTab);
      await tester.tap(dialogVoiceTab);
      await tester.pumpAndSettle();
      final openTextVoice = find.byKey(
        const Key('revision3-npc-greeting-open-text-voice'),
      );
      final outerGreetingScroll = find
          .ancestor(of: openTextVoice, matching: find.byType(Scrollable))
          .last;
      await tester.scrollUntilVisible(
        openTextVoice,
        240,
        scrollable: outerGreetingScroll,
      );
      await tester.ensureVisible(openTextVoice);
      await tester.pumpAndSettle();
      expect(openTextVoice.hitTestable(), findsOneWidget);
      await tester.tap(openTextVoice);
      await tester.pumpAndSettle();

      expect(find.byType(Revision3LocalizationVoiceWorkspace), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-production-no-slot')),
        findsOneWidget,
      );
      expect(find.text('Language: de'), findsOneWidget);
      expect(find.text('Asghan — Mine entrance question'), findsOneWidget);
      final addTake = find.byKey(const Key('revision3-voice-production-add'));
      final planRecording = find.byKey(
        const Key('revision3-voice-production-plan'),
      );
      await _scrollManagedEditorUntilVisible(tester, planRecording);
      expect(addTake, findsOneWidget);
      expect(tester.widget<FilledButton>(addTake).onPressed, isNotNull);
      expect(planRecording, findsOneWidget);
      expect(tester.widget<OutlinedButton>(planRecording).onPressed, isNotNull);
      for (final technicalIdentity in const <String>[
        revision3VoiceContentProjectId,
        revision3VoiceContentLocalizationId,
        revision3VoiceContentLineId,
        _homeNpcGreetingNpcId,
      ]) {
        expect(find.textContaining(technicalIdentity), findsNothing);
      }

      await tester.tap(planRecording);
      await tester.pumpAndSettle();

      expect(managed.dialogVoiceSlotCreationCalls, 1);
      expect(managed.projectRevision, 8);
      expect(publishedPlan, isNotNull);
      expect(
        find.text(
          'Recording planned. An empty Voice setup was added for this line and language. No audio, game file, or save was changed; build and runtime remain unqualified.',
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-intact')),
        findsOneWidget,
      );
      expect(find.text('0 takes'), findsOneWidget);
      expect(find.text('Language: de'), findsOneWidget);
      expect(find.text('Asghan — Mine entrance question'), findsOneWidget);
      expect(planRecording, findsNothing);

      final mode = find.byKey(const Key('revision3-localization-voice-mode'));
      final workspace = tester.widget<Revision3LocalizationVoiceWorkspace>(
        find.byType(Revision3LocalizationVoiceWorkspace),
      );
      final workListMode = find.descendant(
        of: mode,
        matching: find.text(workspace.voiceProductionQueueCopy.title),
      );
      await tester.ensureVisible(workListMode);
      await tester.tap(workListMode);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-localization-voice-work-list')),
        findsOneWidget,
      );
      const itemKey = 'voice:$revision3VoiceContentLineId:de';
      final addRecording = find.byKey(
        const ValueKey('revision3-voice-production-queue-action-$itemKey'),
      );
      expect(addRecording, findsOneWidget);
      expect(tester.widget<FilledButton>(addRecording).onPressed, isNotNull);
      expect(
        find.descendant(of: addRecording, matching: find.text('Add recording')),
        findsOneWidget,
      );
      for (final technicalIdentity in <String>[
        revision3VoiceContentProjectId,
        revision3VoiceContentLocalizationId,
        revision3VoiceContentLineId,
        _homeNpcGreetingNpcId,
        plannedSlotId!,
      ]) {
        expect(find.textContaining(technicalIdentity), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'no game configuration keeps planning available while Add take stays unavailable',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeDialogVoiceSlotCreationManagedLease(
        root: Directory(r'C:\mods\voice-plan-without-game'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _voiceLocalizationWorkspaceIndex(
          revision: lease.projectRevision,
          existingDeSlot: false,
        ),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: localizationId,
                  localizationRevision: localizationRevision,
                  locId: requestedLocId,
                  lineId: revision3VoiceContentLineId,
                  lineDisplayName: 'Mine entrance question',
                  speaker: 'Asghan',
                  voiceSlotLocales: const <String>{},
                  germanText: 'Willkommen am Mineneingang.',
                ),
        onDialogVoiceSlotCreation: (lease, plan) => throw StateError(
          'the no-game availability test must not publish recording intent',
        ),
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
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      await _switchManagedLocalizationVoiceToProjectTexts(tester);

      expect(
        find.byKey(const Key('revision3-voice-production-no-slot')),
        findsOneWidget,
      );
      final planRecording = find.byKey(
        const Key('revision3-voice-production-plan'),
      );
      await _scrollManagedEditorUntilVisible(tester, planRecording);
      expect(planRecording, findsOneWidget);
      expect(tester.widget<OutlinedButton>(planRecording).onPressed, isNotNull);
      expect(
        find.byKey(const Key('revision3-voice-production-add')),
        findsNothing,
      );
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );
      expect(find.textContaining('Gothic 1 Remake installation'), findsWidgets);
      expect(managed.dialogVoiceSlotCreationCalls, 0);
      for (final technicalIdentity in const <String>[
        revision3VoiceContentProjectId,
        revision3VoiceContentLocalizationId,
        revision3VoiceContentLineId,
      ]) {
        expect(find.textContaining(technicalIdentity), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'mismatched planning receipt fails closed and requires a project reopen',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeDialogVoiceSlotCreationManagedLease(
        root: Directory(r'C:\mods\voice-plan-mismatched-receipt'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _voiceLocalizationWorkspaceIndex(
          revision: lease.projectRevision,
          existingDeSlot: false,
        ),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, requestedLocId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: localizationId,
                  localizationRevision: localizationRevision,
                  locId: requestedLocId,
                  lineId: revision3VoiceContentLineId,
                  lineDisplayName: 'Mine entrance question',
                  speaker: 'Asghan',
                  voiceSlotLocales: const <String>{},
                ),
        onDialogVoiceSlotCreation: (lease, plan) =>
            Revision3DialogVoiceSlotCreationPublication(
              projectId: lease.projectId,
              projectRevision: lease.projectRevision + 2,
              lineId: plan.lineId,
              lineRevision: plan.expectedLineRevision + 1,
              localizationId: plan.localizationId,
              localizationRevision: plan.expectedLocalizationRevision,
              slotId: plan.slotId,
              slotRevision: 0,
              locale: plan.locale,
              locId: plan.locId,
              targetResolution:
                  Revision3ContentVoiceTargetResolution.unresolved,
            ),
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
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      await _switchManagedLocalizationVoiceToProjectTexts(tester);
      final planRecording = find.byKey(
        const Key('revision3-voice-production-plan'),
      );
      await _scrollManagedEditorUntilVisible(tester, planRecording);
      await tester.tap(planRecording);
      await tester.pumpAndSettle();

      expect(managed.dialogVoiceSlotCreationCalls, 1);
      expect(managed.projectRevision, 7);
      expect(managed.requiresReopen, isTrue);
      expect(
        find.text(
          'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('Recording planned.'), findsNothing);
      expect(planRecording, findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'direct Story publish refreshes the provider and selects the exact new Quest',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_story_exact_publish_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '59595959595959595959595959595959';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-exact-publish'),
        projectId: projectId,
        projectRevision: 3,
        head: _head(3),
        contentIndexBuilder: (lease) => lease.projectRevision == 3
            ? _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              )
            : _storyWorkbenchGameGateIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              ),
        onQuestPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.title, 'Select the exact Quest');
          lease.projectRevision = 4;
          lease.head = _head(4);
          return Revision3QuestDraftPublication(
            projectId: lease.projectId,
            projectRevision: 4,
            questId: revision3QuestOutlineQuestId,
            scriptModuleId: revision3QuestOutlineModuleId,
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
        loadQuestCatalog: (_) async => _questCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      expect(
        find.byKey(const Key('revision3-story-workspace-empty')),
        findsOneWidget,
      );

      await _openAdvancedQuestCreation(tester);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.enterText(
        find.byKey(const Key('revision3-quest-title')),
        'Select the exact Quest',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-description')),
        'Refresh the provider and keep authoring in Story.',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-objective')),
        'Open the newly published Quest',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await tester.pumpAndSettle();

      final providerState = container.read(currentProjectCoordinatorProvider);
      expect(providerState, isA<ManagedRevision3CurrentProjectState>());
      expect(
        (providerState as ManagedRevision3CurrentProjectState).projectRevision,
        4,
      );
      expect(providerState.head.canonicalJson, _head(4).canonicalJson);
      expect(managed.questPublishCalls, 1);
      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
        ),
      );
      expect(selectedQuest, findsOneWidget);
      expect(tester.widget<ListTile>(selectedQuest).selected, isTrue);
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-'
            '$projectId-$revision3QuestOutlineQuestId',
          ),
        ),
        findsOneWidget,
      );
      expect(
        tester
            .widget<Text>(
              find.byKey(
                const Key('revision3-story-workspace-checkpoint-summary'),
              ),
            )
            .data,
        contains('project revision 4'),
      );
      expect(
        find.textContaining(
          'could not be selected at its exact project revision',
        ),
        findsNothing,
      );
    },
  );

  testWidgets(
    'an awaiting Story create modal cannot select into a switched project',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_story_switched_modal_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const firstProjectId = '61616161616161616161616161616161';
      const secondProjectId = '62626262626262626262626262626262';
      final first = _FakeManagedLease(
        root: Directory(r'C:\mods\story-modal-first'),
        projectId: firstProjectId,
        projectRevision: 3,
        head: _head(3),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final second = _FakeManagedLease(
        root: Directory(r'C:\mods\story-modal-second'),
        projectId: secondProjectId,
        projectRevision: 4,
        head: _head(4),
        contentIndexBuilder: (lease) => _storyWorkbenchGameGateIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async {
          if (root.path == first.root.path) return first;
          if (root.path == second.root.path) return second;
          throw StateError('unexpected managed project ${root.path}');
        },
      );
      await coordinator.openManagedRevision3(first.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadQuestCatalog: (_) async => _questCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-story'),
      );
      await _openAdvancedQuestCreation(tester);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      final oldWizard = find.byKey(const Key('revision3-quest-wizard'));
      expect(oldWizard, findsOneWidget);

      await coordinator.openManagedRevision3(second.root);
      await tester.pumpAndSettle();
      expect(
        (container.read(currentProjectCoordinatorProvider)
                as ManagedRevision3CurrentProjectState)
            .projectId,
        secondProjectId,
      );
      _selectManagedWorkspaceTabProgrammatically(
        tester,
        Revision3ProjectWorkspaceSection.story,
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-'
            '$secondProjectId-$revision3NpcInspectionNpcId',
          ),
          skipOffstage: false,
        ),
        findsOneWidget,
      );

      Navigator.of(tester.element(oldWizard)).pop(
        Revision3QuestDraftPublication(
          projectId: firstProjectId,
          projectRevision: 4,
          questId: revision3QuestOutlineQuestId,
          scriptModuleId: revision3QuestOutlineModuleId,
        ),
      );
      await tester.pumpAndSettle();

      final secondNpc = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3NpcInspectionNpcId',
        ),
      );
      final sameIdQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
        ),
      );
      expect(tester.widget<ListTile>(secondNpc).selected, isTrue);
      expect(tester.widget<ListTile>(sameIdQuest).selected, isFalse);
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-'
            '$secondProjectId-$revision3NpcInspectionNpcId',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-'
            '$secondProjectId-$revision3QuestOutlineQuestId',
          ),
        ),
        findsNothing,
      );
      expect(first.questPublishCalls, 0);
      expect(second.questPublishCalls, 0);
    },
  );

  testWidgets(
    'real compact German Home keeps Story copy list and actions usable',
    (tester) async {
      await _setNarrowShortTestSurface(tester);
      final fixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-compact-german'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (_) => fixture.contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: fixture.projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        Localizations.localeOf(
          tester.element(find.byKey(const Key('revision3-project-workspace'))),
        ).languageCode,
        'de',
      );
      await _navigateManagedStory(tester);

      final createNpcOpening = find.byKey(
        const Key('revision3-story-workspace-create-npc-opening'),
      );
      final createQuestOpening = find.byKey(
        const Key('revision3-story-workspace-create-quest-opening'),
      );
      expect(tester.widget<FilledButton>(createNpcOpening).onPressed, isNull);
      expect(tester.widget<FilledButton>(createQuestOpening).onPressed, isNull);
      final missingGame = AppLocalizations.of(
        tester.element(createNpcOpening),
      ).managedDashboardMissingGameDescription;
      expect(
        missingGame,
        'Richte die Gothic-1-Remake-Installation in den Einstellungen ein, '
        'bevor du Aktionen verwendest, die Nachweise aus dem installierten '
        'Spiel benötigen.',
      );
      expect(find.text(missingGame), findsOneWidget);
      final advancedCreate = find.byKey(
        const Key('revision3-story-workspace-create-advanced'),
      );
      await tester.ensureVisible(advancedCreate);
      await tester.pumpAndSettle();
      expect(advancedCreate.hitTestable(), findsOneWidget);
      await tester.tap(advancedCreate);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .enabled,
        isFalse,
      );
      expect(
        tester
            .widget<PopupMenuItem<dynamic>>(
              find.byKey(const Key('revision3-story-workspace-create-quest')),
            )
            .enabled,
        isFalse,
      );
      await tester.tapAt(const Offset(2, 2));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);

      final quest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
        ),
      );
      await tester.ensureVisible(quest);
      await tester.pumpAndSettle();
      expect(quest.hitTestable(), findsOneWidget);
      await tester.tap(quest);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      final edit = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      expect(edit, findsOneWidget);
      expect(find.text('Name & Ziele bearbeiten'), findsOneWidget);
      await tester.ensureVisible(edit);
      await tester.pump();
      expect(tester.widget<OutlinedButton>(edit).onPressed, isNotNull);
      await tester.tap(edit);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('revision3-quest-outline-dialog')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('revision3-quest-outline-cancel')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Problems opens the exact referenced entity in Story checks', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final fixture = revision3ProjectProblemsFixture(
      includeDialogGraph: false,
      includeReferenceProblem: true,
      includeAssetProblem: false,
      includeVoiceProblems: false,
      includeDataAssetStage: false,
    );
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\problems-deep-link'),
      projectId: fixture.projectId,
      projectRevision: fixture.projectRevision,
      head: _head(fixture.projectRevision),
      contentIndexBuilder: (_) => fixture.contentIndex,
      onDataAssetList: (_) => fixture.dataAssetStages,
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
    await _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-tab-validate-test'),
    );

    final openEntity = find.byKey(
      const Key(
        'revision3-project-problems-action-entity-'
        '$revision3ProjectProblemsNpcId',
      ),
    );
    expect(openEntity, findsOneWidget);
    await tester.ensureVisible(openEntity);
    await tester.tap(openEntity);
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-story-workspace')), findsOneWidget);
    expect(
      find.byKey(
        const Key(
          'revision3-content-entity-details-'
          '$revision3ProjectProblemsNpcId',
        ),
      ),
      findsOneWidget,
    );
    final problemsTab = find.byKey(
      const Key(
        'revision3-story-workbench-tab-problemsChecks-'
        '$revision3ProjectProblemsNpcId',
      ),
    );
    expect(problemsTab, findsOneWidget);
    expect(tester.widget<ChoiceChip>(problemsTab).selected, isTrue);
    expect(
      find.byKey(
        const Key(
          'revision3-story-workbench-section-problemsChecks-'
          '$revision3ProjectProblemsNpcId',
        ),
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    'Problems rendered entity action rejects same-revision head replacement',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = revision3ProjectProblemsFixture(
        includeDialogGraph: false,
        includeReferenceProblem: true,
        includeAssetProblem: false,
        includeVoiceProblems: false,
        includeDataAssetStage: false,
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\problems-rendered-head-a'),
        projectId: fixture.projectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (_) => fixture.contentIndex,
        onDataAssetList: (_) => fixture.dataAssetStages,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );

      final openEntity = find.byKey(
        const Key(
          'revision3-project-problems-action-entity-'
          '$revision3ProjectProblemsNpcId',
        ),
      );
      await tester.ensureVisible(openEntity);
      await tester.pump();
      final renderedAction = tester.widget<FilledButton>(openEntity).onPressed;
      expect(renderedAction, isNotNull);

      final replacementHead = _head(fixture.projectRevision + 1);
      managed.head = replacementHead;
      (coordinator as dynamic).state = ManagedRevision3CurrentProjectState(
        root: managed.root,
        projectId: managed.projectId,
        projectRevision: managed.projectRevision,
        head: replacementHead,
        requiresReopen: false,
      );
      renderedAction!();
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-story-workspace')), findsNothing);
      expect(
        find.byKey(const Key('revision3-content-workspace-navigation')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-project-problems-view')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Problems rendered entity action rejects another root at the same head',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = revision3ProjectProblemsFixture(
        includeDialogGraph: false,
        includeReferenceProblem: true,
        includeAssetProblem: false,
        includeVoiceProblems: false,
        includeDataAssetStage: false,
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\problems-rendered-root-a'),
        projectId: fixture.projectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (_) => fixture.contentIndex,
        onDataAssetList: (_) => fixture.dataAssetStages,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );

      final openEntity = find.byKey(
        const Key(
          'revision3-project-problems-action-entity-'
          '$revision3ProjectProblemsNpcId',
        ),
      );
      await tester.ensureVisible(openEntity);
      await tester.pump();
      final renderedAction = tester.widget<FilledButton>(openEntity).onPressed;
      expect(renderedAction, isNotNull);

      (coordinator as dynamic).state = ManagedRevision3CurrentProjectState(
        root: Directory(r'C:\mods\problems-rendered-root-b'),
        projectId: managed.projectId,
        projectRevision: managed.projectRevision,
        head: managed.head,
        requiresReopen: false,
      );
      renderedAction!();
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-story-workspace')), findsNothing);
      expect(
        find.byKey(const Key('revision3-content-workspace-navigation')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Problems entity handoff rejects delayed same-revision head drift',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = revision3ProjectProblemsFixture(
        includeDialogGraph: false,
        includeReferenceProblem: true,
        includeAssetProblem: false,
        includeVoiceProblems: false,
        includeDataAssetStage: false,
      );
      final delayedEntityLoad = Completer<Revision3ContentIndex>();
      var reads = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\problems-head-drift'),
        projectId: fixture.projectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onContentIndexRead: (_) {
          reads++;
          return switch (reads) {
            1 => Future<Revision3ContentIndex>.value(fixture.contentIndex),
            2 => Future<Revision3ContentIndex>.value(fixture.contentIndex),
            3 => delayedEntityLoad.future,
            _ => Future<Revision3ContentIndex>.value(fixture.contentIndex),
          };
        },
        onDataAssetList: (_) => fixture.dataAssetStages,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );
      expect(reads, 2);
      final openEntity = find.byKey(
        const Key(
          'revision3-project-problems-action-entity-'
          '$revision3ProjectProblemsNpcId',
        ),
      );
      await tester.ensureVisible(openEntity);
      await tester.pump();
      await tester.tap(openEntity);
      await tester.pump();
      expect(reads, 3);

      final driftedHead = _head(fixture.projectRevision + 1);
      managed.head = driftedHead;
      (coordinator as dynamic).state = ManagedRevision3CurrentProjectState(
        root: managed.root,
        projectId: managed.projectId,
        projectRevision: managed.projectRevision,
        head: driftedHead,
        requiresReopen: false,
      );
      await tester.pump();
      delayedEntityLoad.complete(fixture.contentIndex);
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-story-workspace')), findsNothing);
      expect(
        find.byKey(const Key('revision3-project-problems-view')),
        findsOneWidget,
      );
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .head
            .canonicalJson,
        driftedHead.canonicalJson,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Problems opens and expands the exact DataAsset edit', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final fixture = revision3ProjectProblemsFixture(
      includeDialogGraph: false,
      includeReferenceProblem: false,
      includeAssetProblem: false,
      includeVoiceProblems: false,
      includeDataAssetStage: true,
    );
    final stage = fixture.dataAssetStage!;
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: true,
    );
    final stageProblem = report.problems.singleWhere(
      (problem) =>
          problem.code == Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
    );
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\problems-dataasset-deep-link'),
      projectId: fixture.projectId,
      projectRevision: fixture.projectRevision,
      head: _head(fixture.projectRevision),
      contentIndexBuilder: (_) => fixture.contentIndex,
      onDataAssetList: (_) => fixture.dataAssetStages,
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
    await _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-tab-validate-test'),
    );
    await tester.tap(
      find.byKey(Key('revision3-project-problem-${stageProblem.id}')),
    );
    await tester.pump();
    final openStage = find.byKey(
      Key(
        'revision3-project-problems-action-dataAssetStage-${stage.targetPath}',
      ),
    );
    await tester.ensureVisible(openStage);
    await tester.tap(openStage);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-dataasset-stage-panel')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-dataasset-stage-search')),
          )
          .controller!
          .text,
      stage.targetPath,
    );
    expect(
      find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      findsOneWidget,
    );
    expect(
      find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'Validate Voice blocker opens the exact line and locale workflow',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_validate_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-validate'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            revision3VoiceContentIndexFixture(revision: lease.projectRevision),
        onVoicePlan: _unresolvedVoicePlan,
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
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );

      expect(
        find.byKey(const Key('revision3-voice-readiness-panel')),
        findsOneWidget,
      );
      expect(find.text('0 of 1 Voice slots are ready.'), findsOneWidget);
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      await tester.tap(
        find.byKey(const Key('revision3-voice-readiness-toggle-blockers')),
      );
      await tester.pumpAndSettle();
      final resolve = find.byKey(
        const ValueKey('revision3-voice-readiness-blocker-action-0'),
      );
      await tester.ensureVisible(resolve);
      await tester.tap(resolve);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-voice-target-dialog')),
        findsOneWidget,
      );
      expect(find.text('Current target: unresolved'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-target-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(find.byType(DropdownButtonFormField<String>), findsNothing);
      expect(find.text(revision3VoiceContentLineId), findsNothing);

      await tester.tap(find.byKey(const Key('revision3-voice-target-cancel')));
      await tester.pumpAndSettle();
      expect(managed.voicePlanCalls, 2);
    },
  );

  testWidgets(
    'German managed Home localizes Voice readiness and the plan-first build',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_validate_de_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-validate-de'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            revision3VoiceContentIndexFixture(revision: lease.projectRevision),
        onVoicePlan: _readyVoicePlan,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      String? pickerLabel;
      final container = _container(
        coordinator: coordinator,
        pickManaged: (label) async {
          pickerLabel = label;
          return null;
        },
        gamePath: gameRoot.path,
      );
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-validate-test'),
      );

      expect(find.text('Voice-Bereitschaft'), findsOneWidget);
      expect(find.text('Voice ist bereit'), findsOneWidget);
      expect(find.text('1 von 1 Voice-Slots sind bereit.'), findsOneWidget);
      expect(find.text('Voice readiness'), findsNothing);
      expect(find.text('Voice is ready'), findsNothing);

      final build = find.byKey(const Key('revision3-voice-readiness-build'));
      await tester.ensureVisible(build);
      await tester.tap(build);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-voice-build-dialog')),
        findsOneWidget,
      );
      expect(find.text('Voice-Bundle bauen'), findsOneWidget);
      expect(find.text('Name des neuen Ordners'), findsOneWidget);
      expect(find.text('Übergeordneten Ordner wählen'), findsOneWidget);
      expect(
        find.textContaining('Dadurch wird ein versiegeltes Voice-Bundle'),
        findsOneWidget,
      );
      expect(find.text('Build Voice bundle'), findsNothing);
      expect(find.text('New folder name'), findsNothing);
      expect(find.textContaining('Offline build only'), findsNothing);
      expect(managed.voicePlanCalls, 2);

      await tester.tap(
        find.byKey(const Key('revision3-voice-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      expect(pickerLabel, 'Übergeordneten Ordner für das Voice-Bundle wählen');

      final reportDeepLinkFailure = tester
          .widget<Revision3VoiceBuildDialog>(
            find.byType(Revision3VoiceBuildDialog),
          )
          .onDeepLinkFailure;
      await tester.tap(find.byKey(const Key('revision3-voice-build-close')));
      await tester.pumpAndSettle();
      await Future<void>.sync(reportDeepLinkFailure);
      await tester.pump();

      expect(
        find.text(
          'Der ausgewählte Voice-Workflow konnte nicht geöffnet werden. '
          'Aktualisiere die Ansicht und versuche es erneut.',
        ),
        findsOneWidget,
      );
      expect(find.byType(SnackBar), findsOneWidget);
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

      await _tapManagedHomeTask(tester, const Key('managed-home-story'));
      expect(
        find.byKey(const Key('revision3-story-workspace')),
        findsOneWidget,
      );
      expect(managed.contentReadCalls, 2);

      await _openAdvancedQuestCreation(tester);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
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
      expect(managed.contentReadCalls, 4);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      await _expandManagedTechnicalDetails(tester);
      expect(find.byKey(const Key('managed-project-revision')), findsOneWidget);
      expect(find.text('8'), findsWidgets);
      expect(
        find.textContaining(
          'could not be selected at its exact project revision',
        ),
        findsOneWidget,
      );
      expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
    },
  );

  testWidgets(
    'guided Character + first greeting inserts line zero and opens exact Dialog Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_opening_recipe_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '18181818181818181818181818181818';
      const openingRevision = 7;
      const greetingLineName = 'North Gate welcome';
      const greetingLineText = 'Willkommen am Nordtor.';
      final contentRevisions = <int>[];
      Revision3NpcGreetingCreateTechnicalPlan? greetingPlan;
      final managed = _FakeNpcGreetingManagedLease(
        root: Directory(r'C:\mods\npc-opening-recipe'),
        projectId: projectId,
        projectRevision: openingRevision,
        head: _head(openingRevision),
        contentIndexBuilder: (lease) {
          contentRevisions.add(lease.projectRevision);
          return _npcOpeningRecipeContentIndex(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            openingRevision: openingRevision,
            createPlan: greetingPlan,
          );
        },
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.displayName, 'North Gate Guard');
          expect(input.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
          lease.projectRevision = openingRevision + 1;
          lease.head = _head(openingRevision + 1);
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
          );
        },
        onNpcGreetingCreate: (lease, plan) {
          expect(lease.projectRevision, openingRevision + 1);
          expect(plan.npcId, _homeCreatedNpcId);
          expect(plan.expectedNpcRevision, 0);
          expect(plan.expectedModuleId, _homeCreatedNpcModuleId);
          expect(plan.expectedModuleRevision, 0);
          expect(plan.expectedGreetingCount, 0);
          expect(plan.index, 0);
          expect(plan.line.lineDisplayName, greetingLineName);
          expect(plan.line.speakerHint, 'North Gate Guard');
          expect(plan.line.locale, 'de');
          final localization =
              plan.line.localization
                  as AuthoringRevision3DialogLocalizationCreateIntentV1;
          expect(localization.texts, <String, String>{'de': greetingLineText});
          greetingPlan = plan;
          lease.projectRevision = openingRevision + 2;
          lease.head = _head(openingRevision + 2);
          return Revision3NpcGreetingPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            npcId: plan.npcId,
            npcRevision: plan.expectedNpcRevision + 1,
            moduleId: plan.expectedModuleId,
            moduleRevision: plan.expectedModuleRevision,
            mode: AuthoringRevision3NpcGreetingMode.createAndInsert,
            greetingCount: plan.expectedGreetingCount + 1,
            createdLineId: plan.line.lineId,
            createdLocalizationId: localization.localizationId,
            createdVoiceSlotId: plan.line.voiceSlot?.slotId,
            localizationAction:
                AuthoringRevision3DialogLocalizationAction.created,
          );
        },
        onDialogLocalizationRead:
            (lease, localizationId, localizationRevision, locId) {
              final plan = greetingPlan;
              if (plan == null) {
                throw StateError('greeting plan has not been published');
              }
              final localization =
                  plan.line.localization
                      as AuthoringRevision3DialogLocalizationCreateIntentV1;
              expect(localizationId, localization.localizationId);
              expect(localizationRevision, 0);
              expect(locId, localization.locId);
              return _questOpeningRecipeLocalizationReadResult(
                lease: lease,
                line: plan.line,
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
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
        reason: 'global Create must still be able to lazy-mount Story',
      );
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      final recipeAction = find.byKey(
        const Key('managed-project-create-npc-opening'),
      );
      expect(recipeAction, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(recipeAction));
      expect(
        find.text(l10n.managedStoryWorkspaceCreateNpcOpening),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedNpcOpeningRecipeDescription),
        findsOneWidget,
      );
      await tester.tap(recipeAction);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(
        find.byKey(const Key('managed-npc-opening-recipe-intro')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedNpcOpeningRecipeIntroduction),
        findsOneWidget,
      );
      expect(
        l10n.managedNpcOpeningRecipeIntroduction,
        contains(
          'does not create dialog logic, runtime behavior, a spawn, '
          'or change the game or save files',
        ),
      );
      await tester.tap(
        find.byKey(const Key('managed-npc-opening-recipe-start')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'North Gate Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-name')),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedNpcOpeningGreetingIntroduction),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        greetingLineName,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-speaker')),
        'North Gate Guard',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-locale')),
        'de',
      );
      await _dragUntilFound(
        tester,
        scrollable: find.byKey(const Key('revision3-dialog-line-editor')),
        target: find.byKey(const Key('revision3-dialog-line-text')),
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-text')),
        greetingLineText,
      );
      final submit = find.byKey(const Key('revision3-dialog-line-submit'));
      await tester.ensureVisible(submit);
      await tester.tap(submit);
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-success')),
      );

      expect(managed.npcPublishCalls, 1);
      expect(managed.npcGreetingCreateCalls, 1);
      expect(managed.dialogLinePublishCalls, 0);
      expect(greetingPlan, isNotNull);
      await tester.tap(find.byKey(const Key('revision3-dialog-line-done')));
      await tester.pumpAndSettle();

      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, openingRevision + 2);
      expect(
        state.head.canonicalJson,
        _head(openingRevision + 2).canonicalJson,
      );
      expect(contentRevisions.first, openingRevision);
      expect(contentRevisions, contains(openingRevision + 1));
      expect(contentRevisions, contains(openingRevision + 2));
      final completionCopy = l10n.managedNpcOpeningRecipeComplete(
        openingRevision + 2,
      );
      expect(find.text(completionCopy), findsOneWidget);
      expect(
        completionCopy,
        contains('no playable conversation or spawn was created'),
      );
      expect(completionCopy, contains('game and save files were not changed'));

      final selectedNpc = find.byKey(
        const Key('revision3-story-workspace-entity-$_homeCreatedNpcId'),
      );
      expect(selectedNpc, findsOneWidget);
      expect(tester.widget<ListTile>(selectedNpc).selected, isTrue);
      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-$_homeCreatedNpcId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-npc-dialog-voice-panel')),
        findsOneWidget,
      );
      final selectedGreeting = find.byKey(
        const Key('revision3-npc-greeting-row-0'),
      );
      expect(selectedGreeting, findsOneWidget);
      expect(tester.widget<ListTile>(selectedGreeting).selected, isTrue);
      expect(find.text(greetingLineName), findsWidgets);
      expect(find.text(greetingLineText), findsOneWidget);

      final plan = greetingPlan!;
      final localization =
          plan.line.localization
              as AuthoringRevision3DialogLocalizationCreateIntentV1;
      for (final technicalId in <String>[
        _homeCreatedNpcId,
        _homeCreatedNpcModuleId,
        _homeCreatedNpcUniqueName,
        _homeCreatedNpcModuleNamespace,
        plan.line.lineId,
        plan.line.lineAuthoredIdentity,
        localization.localizationId,
        localization.locId,
        if (plan.line.voiceSlot case final slot?) slot.slotId,
      ]) {
        expect(find.text(technicalId), findsNothing);
      }
      for (final unsupportedClaim in const <String>[
        'Playable conversation created',
        'Runtime behavior created',
        'Spawn created',
        'Game files changed',
        'Save files changed',
      ]) {
        expect(find.textContaining(unsupportedClaim), findsNothing);
      }
      expect(find.text('Build / Deploy'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'guided Character + first greeting cancel keeps exact Character-only checkpoint',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_opening_recipe_cancel_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '17171717171717171717171717171717';
      const openingRevision = 7;
      final contentRevisions = <int>[];
      final managed = _FakeNpcGreetingManagedLease(
        root: Directory(r'C:\mods\npc-opening-recipe-cancel'),
        projectId: projectId,
        projectRevision: openingRevision,
        head: _head(openingRevision),
        contentIndexBuilder: (lease) {
          contentRevisions.add(lease.projectRevision);
          return _npcOpeningRecipeContentIndex(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            openingRevision: openingRevision,
          );
        },
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.displayName, 'Keep this Character');
          lease.projectRevision = openingRevision + 1;
          lease.head = _head(openingRevision + 1);
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
          );
        },
        onNpcGreetingCreate: (_, _) => throw TestFailure(
          'cancelling the greeting dialog must not publish a greeting',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _tapManagedHomeTask(tester, const Key('managed-home-story'));

      final recipeAction = find.byKey(
        const Key('revision3-story-workspace-create-npc-opening'),
      );
      expect(recipeAction, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(recipeAction));
      expect(
        find.text(l10n.managedStoryWorkspaceCreateNpcOpening),
        findsOneWidget,
      );
      await tester.tap(recipeAction);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.byKey(const Key('managed-npc-opening-recipe-intro')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('managed-npc-opening-recipe-start')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Keep this Character',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-modal')),
      );
      await tester.tap(find.byKey(const Key('revision3-dialog-line-cancel')));
      await tester.pumpAndSettle();

      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, openingRevision + 1);
      expect(
        state.head.canonicalJson,
        _head(openingRevision + 1).canonicalJson,
      );
      expect(managed.npcPublishCalls, 1);
      expect(managed.npcGreetingCreateCalls, 0);
      expect(managed.dialogLinePublishCalls, 0);
      expect(contentRevisions.first, openingRevision);
      expect(contentRevisions, contains(openingRevision + 1));
      expect(contentRevisions, isNot(contains(openingRevision + 2)));
      expect(
        find.text(l10n.managedNpcOpeningRecipePartial(openingRevision + 1)),
        findsOneWidget,
      );

      final selectedNpc = find.byKey(
        const Key('revision3-story-workspace-entity-$_homeCreatedNpcId'),
      );
      expect(selectedNpc, findsOneWidget);
      expect(tester.widget<ListTile>(selectedNpc).selected, isTrue);
      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-$_homeCreatedNpcId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-npc-dialog-voice-panel')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-npc-greeting-row-0')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-npc-greeting-new-line')),
        findsOneWidget,
      );
      for (final technicalId in const <String>[
        _homeCreatedNpcId,
        _homeCreatedNpcModuleId,
        _homeCreatedNpcUniqueName,
        _homeCreatedNpcModuleNamespace,
      ]) {
        expect(find.text(technicalId), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'locked Character opening ignores rejected NPC receipt for Story navigation',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_opening_rejected_receipt_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '16161616161616161616161616161616';
      const openingRevision = 7;
      const genericNpcId = '91919191919191919191919191919191';
      const rejectedNpcId = '99999999999999999999999999999999';
      const rejectedModuleId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
      final managed = _FakeNpcGreetingManagedLease(
        root: Directory(r'C:\mods\npc-opening-rejected-receipt'),
        projectId: projectId,
        projectRevision: openingRevision,
        head: _head(openingRevision),
        contentIndexBuilder: (lease) => _globalSearchContentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          targetEntityId: rejectedNpcId,
        ),
        onNpcPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.displayName, 'Rejected receipt guard');
          lease.projectRevision = openingRevision + 1;
          lease.head = _head(openingRevision + 1);
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            head: lease.head,
            npcId: rejectedNpcId,
            scriptModuleId: rejectedModuleId,
          );
        },
        onNpcGreetingCreate: (_, _) => throw TestFailure(
          'a rejected NPC receipt must lock before Greeting authoring',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
      );
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      final recipeAction = find.byKey(
        const Key('managed-project-create-npc-opening'),
      );
      final l10n = AppLocalizations.of(tester.element(recipeAction));
      await tester.tap(recipeAction);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('managed-npc-opening-recipe-start')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Rejected receipt guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pump();

      expect(managed.npcPublishCalls, 1);
      var current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, openingRevision + 1);
      expect(
        current.head.canonicalJson,
        _head(openingRevision + 1).canonicalJson,
      );

      // Poison only the checkpoint captured for the recipe step. The valid
      // publication no longer matches this rebound head, so the recipe returns
      // a LockedOutcome carrying an explicitly rejected non-null NPC step.
      managed.head = _head(108);
      await coordinator.verifyCurrent();
      await tester.pumpAndSettle();

      current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, openingRevision + 1);
      expect(current.head.canonicalJson, _head(108).canonicalJson);
      expect(managed.npcGreetingCreateCalls, 0);
      expect(find.text(l10n.managedNpcOpeningRecipeStopped), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsOneWidget,
      );

      final genericNpc = find.byKey(
        const Key('revision3-story-workspace-entity-$genericNpcId'),
      );
      final rejectedNpc = find.byKey(
        const Key('revision3-story-workspace-entity-$rejectedNpcId'),
      );
      expect(genericNpc, findsOneWidget);
      expect(tester.widget<ListTile>(genericNpc).selected, isTrue);
      expect(rejectedNpc, findsOneWidget);
      expect(tester.widget<ListTile>(rejectedNpc).selected, isFalse);
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-$projectId-$genericNpcId',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key(
            'revision3-story-workspace-workbench-$projectId-$rejectedNpcId',
          ),
        ),
        findsNothing,
      );
      expect(
        find.byKey(
          const Key('revision3-story-workbench-tab-dialogVoice-$rejectedNpcId'),
        ),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'guided Quest opening recipe inserts line zero and opens exact Dialog Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_quest_opening_recipe_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '19191919191919191919191919191919';
      const openingRevision = 7;
      const openingLineName = 'Asghan opening warning';
      const openingLineText = 'Halt! Bevor du gehst, hoer mir zu.';
      final contentRevisions = <int>[];
      Revision3QuestTranscriptCreateTechnicalPlan? transcriptPlan;
      final managed = _FakeQuestTranscriptManagedLease(
        root: Directory(r'C:\mods\quest-opening-recipe'),
        projectId: projectId,
        projectRevision: openingRevision,
        head: _head(openingRevision),
        contentIndexBuilder: (lease) {
          contentRevisions.add(lease.projectRevision);
          return _questOpeningRecipeContentIndex(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            openingRevision: openingRevision,
            createPlan: transcriptPlan,
          );
        },
        onQuestPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.title, 'Warn Asghan');
          expect(input.parentCatalogId, 'parent-one');
          expect(input.giverCatalogId, 'giver-asghan');
          lease.projectRevision = openingRevision + 1;
          lease.head = _head(openingRevision + 1);
          return Revision3QuestDraftPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: _homeQuestOpeningRecipeQuestId,
            scriptModuleId: _homeQuestOpeningRecipeModuleId,
          );
        },
        onQuestTranscriptCreate: (lease, plan) {
          expect(lease.projectRevision, openingRevision + 1);
          expect(plan.questId, _homeQuestOpeningRecipeQuestId);
          expect(plan.expectedQuestRevision, 4);
          expect(plan.expectedModuleId, _homeQuestOpeningRecipeModuleId);
          expect(plan.expectedModuleRevision, 5);
          expect(plan.expectedTranscriptCount, 0);
          expect(plan.index, 0);
          expect(plan.objectiveSlot, isNull);
          expect(plan.line.lineDisplayName, openingLineName);
          expect(plan.line.speakerHint, 'Asghan');
          expect(plan.line.locale, 'de');
          final localization =
              plan.line.localization
                  as AuthoringRevision3DialogLocalizationCreateIntentV1;
          expect(localization.texts, <String, String>{'de': openingLineText});
          transcriptPlan = plan;
          lease.projectRevision = openingRevision + 2;
          lease.head = _head(openingRevision + 2);
          return Revision3QuestTranscriptPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: plan.questId,
            questRevision: plan.expectedQuestRevision + 1,
            moduleId: plan.expectedModuleId,
            moduleRevision: plan.expectedModuleRevision,
            mode: AuthoringRevision3QuestTranscriptMode.createAndInsert,
            transcriptCount: plan.expectedTranscriptCount + 1,
            createdLineId: plan.line.lineId,
            createdLocalizationId: localization.localizationId,
            createdVoiceSlotId: plan.line.voiceSlot?.slotId,
            localizationAction:
                AuthoringRevision3DialogLocalizationAction.created,
          );
        },
        onDialogLocalizationRead:
            (lease, localizationId, localizationRevision, locId) {
              final plan = transcriptPlan;
              if (plan == null) {
                throw StateError('opening-line plan has not been published');
              }
              final localization =
                  plan.line.localization
                      as AuthoringRevision3DialogLocalizationCreateIntentV1;
              expect(localizationId, localization.localizationId);
              expect(localizationRevision, 0);
              expect(locId, localization.locId);
              return _questOpeningRecipeLocalizationReadResult(
                lease: lease,
                line: plan.line,
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
        loadQuestCatalog: (_) async => _questCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      await _tapManagedHomeTask(tester, const Key('managed-home-story'));
      final recipeAction = find.byKey(
        const Key('revision3-story-workspace-create-quest-opening'),
      );
      expect(recipeAction, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(recipeAction));
      await tester.tap(recipeAction);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.byKey(const Key('managed-quest-opening-recipe-intro')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedQuestOpeningRecipeIntroduction),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('managed-quest-opening-recipe-start')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byKey(const Key('revision3-quest-wizard')), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-quest-title')),
        'Warn Asghan',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-description')),
        'Give Asghan an opening warning before the route continues.',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-objective')),
        'Listen to Asghan',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-name')),
      );

      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedQuestOpeningLineIntroduction),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        openingLineName,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-speaker')),
        'Asghan',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-locale')),
        'de',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-text')),
        openingLineText,
      );
      final submit = find.byKey(const Key('revision3-dialog-line-submit'));
      await tester.ensureVisible(submit);
      await tester.tap(submit);
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-success')),
      );

      expect(
        find.byKey(const Key('revision3-dialog-line-success')),
        findsOneWidget,
      );
      expect(managed.questPublishCalls, 1);
      expect(managed.questTranscriptCreateCalls, 1);
      expect(managed.dialogLinePublishCalls, 0);
      expect(transcriptPlan, isNotNull);

      await tester.tap(find.byKey(const Key('revision3-dialog-line-done')));
      await tester.pumpAndSettle();

      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, openingRevision + 2);
      expect(
        state.head.canonicalJson,
        _head(openingRevision + 2).canonicalJson,
      );
      expect(contentRevisions.first, openingRevision);
      expect(contentRevisions, contains(openingRevision + 1));
      expect(contentRevisions, contains(openingRevision + 2));
      expect(
        find.text(l10n.managedQuestOpeningRecipeComplete(openingRevision + 2)),
        findsOneWidget,
      );

      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-'
          '$_homeQuestOpeningRecipeQuestId',
        ),
      );
      expect(selectedQuest, findsOneWidget);
      expect(tester.widget<ListTile>(selectedQuest).selected, isTrue);
      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$_homeQuestOpeningRecipeQuestId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-quest-transcript-panel')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-transcript-row-0')),
        findsOneWidget,
      );
      expect(find.text(openingLineName), findsWidgets);
      expect(find.text(openingLineText), findsOneWidget);

      final plan = transcriptPlan!;
      final localization =
          plan.line.localization
              as AuthoringRevision3DialogLocalizationCreateIntentV1;
      for (final technicalId in <String>[
        _homeQuestOpeningRecipeQuestId,
        _homeQuestOpeningRecipeModuleId,
        _homeQuestOpeningRecipeTechnicalId,
        plan.line.lineId,
        plan.line.lineAuthoredIdentity,
        localization.localizationId,
        localization.locId,
        if (plan.line.voiceSlot case final slot?) slot.slotId,
      ]) {
        expect(find.text(technicalId), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'guided Quest opening recipe cancel keeps exact Quest-only checkpoint',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_quest_opening_recipe_cancel_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '20202020202020202020202020202020';
      const openingRevision = 7;
      final contentRevisions = <int>[];
      final managed = _FakeQuestTranscriptManagedLease(
        root: Directory(r'C:\mods\quest-opening-recipe-cancel'),
        projectId: projectId,
        projectRevision: openingRevision,
        head: _head(openingRevision),
        contentIndexBuilder: (lease) {
          contentRevisions.add(lease.projectRevision);
          return _questOpeningRecipeContentIndex(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            openingRevision: openingRevision,
          );
        },
        onQuestPublish: (lease, requestedGameRoot, input) {
          expect(requestedGameRoot, gameRoot.path);
          expect(input.title, 'Keep this Quest');
          lease.projectRevision = openingRevision + 1;
          lease.head = _head(openingRevision + 1);
          return Revision3QuestDraftPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: _homeQuestOpeningRecipeQuestId,
            scriptModuleId: _homeQuestOpeningRecipeModuleId,
          );
        },
        onQuestTranscriptCreate: (_, _) => throw TestFailure(
          'cancelling the opening-line dialog must not publish a transcript',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadQuestCatalog: (_) async => _questCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      await _tapManagedHomeTask(tester, const Key('managed-home-story'));
      final recipeAction = find.byKey(
        const Key('revision3-story-workspace-create-quest-opening'),
      );
      final l10n = AppLocalizations.of(tester.element(recipeAction));
      await tester.tap(recipeAction);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.byKey(const Key('managed-quest-opening-recipe-intro')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('managed-quest-opening-recipe-start')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      await tester.enterText(
        find.byKey(const Key('revision3-quest-title')),
        'Keep this Quest',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-description')),
        'The Quest must remain after cancelling only its opening line.',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-quest-objective')),
        'Keep the exact Quest checkpoint',
      );
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await _pumpUntilFound(
        tester,
        find.byKey(const Key('revision3-dialog-line-modal')),
      );

      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('revision3-dialog-line-cancel')));
      await tester.pumpAndSettle();

      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, openingRevision + 1);
      expect(
        state.head.canonicalJson,
        _head(openingRevision + 1).canonicalJson,
      );
      expect(managed.questPublishCalls, 1);
      expect(managed.questTranscriptCreateCalls, 0);
      expect(managed.dialogLinePublishCalls, 0);
      expect(contentRevisions.first, openingRevision);
      expect(contentRevisions, contains(openingRevision + 1));
      expect(contentRevisions, isNot(contains(openingRevision + 2)));
      expect(
        find.text(l10n.managedQuestOpeningRecipePartial(openingRevision + 1)),
        findsOneWidget,
      );

      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-'
          '$_homeQuestOpeningRecipeQuestId',
        ),
      );
      expect(selectedQuest, findsOneWidget);
      expect(tester.widget<ListTile>(selectedQuest).selected, isTrue);
      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$_homeQuestOpeningRecipeQuestId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-quest-transcript-panel')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-transcript-row-0')),
        findsNothing,
      );
      for (final technicalId in const <String>[
        _homeQuestOpeningRecipeQuestId,
        _homeQuestOpeningRecipeModuleId,
        _homeQuestOpeningRecipeTechnicalId,
      ]) {
        expect(find.text(technicalId), findsNothing);
      }
      expect(tester.takeException(), isNull);
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
        onQuestTransitionsSeed:
            (
              lease,
              questId,
              expectedQuestRevision,
              expectedModuleId,
              expectedModuleRevision,
            ) => AuthoringRevision3QuestTransitionsSeed.forProject(
              currentProjectJson: currentFixture.projectJson,
              questId: questId,
              expectedQuestRevision: expectedQuestRevision,
              expectedModuleId: expectedModuleId,
              expectedModuleRevision: expectedModuleRevision,
            ),
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3QuestOutlineQuestId);
      final edit = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      expect(edit, findsOneWidget);
      await tester.ensureVisible(edit);
      await tester.tap(edit);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
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
      expect(managed.contentReadCalls, 8);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
      expect(currentFixture.title, 'Find Homer safely');
      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
        ),
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
    'selected Library Quest Source & checks reaches the exact managed lease',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_quest_source_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final fixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-source-inspection'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: fixture.head,
        canonicalProjectJsonValue: fixture.projectJson,
        contentIndexBuilder: (_) => fixture.contentIndex(),
        onQuestSourceInspection: (lease, requestedGameRoot, questId) async {
          expect(requestedGameRoot, gameRoot.path);
          expect(questId, revision3QuestOutlineQuestId);
          return _questSourceInspection(
            fixture: fixture,
            expectedHead: lease.head,
          );
        },
        onManagedCompilerCheck:
            (
              lease,
              entityKind,
              requestedGameRoot,
              entityId,
              expectedEntityRevision,
              expectedModuleId,
              expectedModuleRevision,
            ) async {
              expect(
                entityKind,
                AuthoringRevision3ManagedCompilerEntityKind.questDraft,
              );
              expect(requestedGameRoot, gameRoot.path);
              expect(entityId, revision3QuestOutlineQuestId);
              expect(expectedEntityRevision, fixture.questRevision);
              expect(expectedModuleId, revision3QuestOutlineModuleId);
              expect(expectedModuleRevision, fixture.moduleRevision);
              return _questManagedCompilerReceipt(
                fixture: fixture,
                expectedHead: lease.head,
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3QuestOutlineQuestId);
      final problemsTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-problemsChecks-$revision3QuestOutlineQuestId',
        ),
      );
      expect(problemsTab, findsOneWidget);
      await tester.ensureVisible(problemsTab);
      await tester.tap(problemsTab);
      await tester.pumpAndSettle();
      final inspect = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-inspect-quest_draft-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, inspect);
      expect(_workbenchActionTileWidget(tester, inspect).enabled, isTrue);
      await _tapWorkbenchAction(tester, inspect);
      await tester.pumpAndSettle();

      expect(managed.questSourceInspectionCalls, 1);
      expect(
        find.byKey(const Key('revision3-quest-source-inspection-dialog')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-source-inspection-result')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-source-inspection-error')),
        findsNothing,
      );
      final panel = tester.widget<Revision3ManagedCompilerCheckPanel>(
        find.byType(Revision3ManagedCompilerCheckPanel),
      );
      final receipt = await panel.check();
      expect(receipt.acceptedAtExactCurrent, isTrue);
      expect(managed.managedCompilerCheckCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
      await tester.tap(
        find.byKey(const Key('revision3-quest-source-inspection-close')),
      );
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'selected Library NPC checks reach the exact lease without a game root',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const revision = 7;
      final contentIndex = _npcInspectionIndex(
        projectId: revision3NpcInspectionProjectId,
        revision: revision,
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\npc-source-inspection'),
        projectId: revision3NpcInspectionProjectId,
        projectRevision: revision,
        head: _head(revision),
        contentIndexBuilder: (_) => contentIndex,
        onNpcSourceInspection: (lease, npcId) async {
          expect(npcId, revision3NpcInspectionNpcId);
          return revision3NpcInspectionResult(
            head: lease.head,
            projectJson: revision3NpcInspectionProjectJson(
              projectId: lease.projectId,
              revision: lease.projectRevision,
            ),
            npcId: npcId,
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3NpcInspectionNpcId);
      final problemsTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-problemsChecks-$revision3NpcInspectionNpcId',
        ),
      );
      await tester.ensureVisible(problemsTab);
      await tester.tap(problemsTab);
      await tester.pumpAndSettle();
      final inspectNpc = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-inspect-npc_draft-$revision3NpcInspectionNpcId',
        ),
      );
      await _revealWorkbenchAction(tester, inspectNpc);
      expect(_workbenchActionTileWidget(tester, inspectNpc).enabled, isTrue);
      await _tapWorkbenchAction(tester, inspectNpc);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-npc-profile-dialog')),
        findsOneWidget,
      );
      expect(managed.npcSourceInspectionCalls, 1);
      expect(managed.npcSourceInspectionNpcIds, <String>[
        revision3NpcInspectionNpcId,
      ]);
      expect(
        find.byKey(const Key('revision3-npc-profile-result')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('revision3-npc-profile-dialog')),
          matching: find.text('Build blocked'),
        ),
        findsOneWidget,
      );
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'unconfigured Story Workbench keeps NPC profile but gates game-backed Quest actions',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const revision = 7;
      final contentIndex = _storyWorkbenchGameGateIndex(
        projectId: revision3NpcInspectionProjectId,
        revision: revision,
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-workbench-game-gate'),
        projectId: revision3NpcInspectionProjectId,
        projectRevision: revision,
        head: _head(revision),
        contentIndexBuilder: (_) => contentIndex,
        onNpcSourceInspection: (lease, npcId) async {
          expect(npcId, revision3NpcInspectionNpcId);
          return revision3NpcInspectionResult(
            head: lease.head,
            projectJson: revision3NpcInspectionProjectJson(
              projectId: lease.projectId,
              revision: lease.projectRevision,
            ),
            npcId: npcId,
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3NpcInspectionNpcId);

      const missingGameReason =
          'Configure the Gothic 1 Remake installation in Settings before using actions that need installed-game evidence.';
      final editNpc = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-npc-profile-$revision3NpcInspectionNpcId',
        ),
      );
      await _revealWorkbenchAction(tester, editNpc);
      expect(_workbenchActionTileWidget(tester, editNpc).enabled, isFalse);
      expect(
        find.descendant(
          of: editNpc,
          matching: find.text(missingGameReason, skipOffstage: false),
          skipOffstage: false,
        ),
        findsOneWidget,
      );
      final npcProblemsTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-problemsChecks-$revision3NpcInspectionNpcId',
        ),
      );
      await tester.ensureVisible(npcProblemsTab);
      await tester.tap(npcProblemsTab);
      await tester.pumpAndSettle();
      final inspectNpc = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-inspect-npc_draft-$revision3NpcInspectionNpcId',
        ),
      );
      await _revealWorkbenchAction(tester, inspectNpc);
      expect(_workbenchActionTileWidget(tester, inspectNpc).enabled, isTrue);
      await _tapWorkbenchAction(tester, inspectNpc);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-npc-profile-dialog')),
        findsOneWidget,
      );
      expect(managed.npcSourceInspectionCalls, 1);
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(
          const Key(
            'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
          ),
        ),
      );
      await tester.pumpAndSettle();

      final editStory = find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      );
      expect(editStory, findsOneWidget);
      await tester.ensureVisible(editStory);
      expect(tester.widget<OutlinedButton>(editStory).onPressed, isNull);
      expect(find.text(missingGameReason, skipOffstage: false), findsWidgets);

      final problemsTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-problemsChecks-$revision3QuestOutlineQuestId',
        ),
      );
      await tester.ensureVisible(problemsTab);
      await tester.tap(problemsTab);
      await tester.pumpAndSettle();
      final inspectQuest = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-inspect-quest_draft-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, inspectQuest);
      expect(_workbenchActionTileWidget(tester, inspectQuest).enabled, isFalse);
      expect(
        find.descendant(
          of: inspectQuest,
          matching: find.text(missingGameReason, skipOffstage: false),
          skipOffstage: false,
        ),
        findsOneWidget,
      );
      expect(managed.questContextSeedCalls, 0);
      expect(managed.questSourceInspectionCalls, 0);
      expect(
        find.byKey(const Key('revision3-quest-context-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-quest-source-inspection-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'NPC compiler panel binds the selected exact entity and module to the lease',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_compiler_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final fixture = _npcManagedCompilerFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\npc-managed-compiler'),
        projectId: revision3NpcInspectionProjectId,
        projectRevision: 7,
        head: fixture.head,
        canonicalProjectJsonValue: fixture.projectJson,
        contentIndexBuilder: (_) => fixture.contentIndex,
        onNpcSourceInspection: (lease, npcId) async =>
            revision3NpcInspectionResult(
              head: lease.head,
              projectJson: lease.canonicalProjectJson,
              npcId: npcId,
            ),
        onManagedCompilerCheck:
            (
              lease,
              entityKind,
              requestedGameRoot,
              entityId,
              expectedEntityRevision,
              expectedModuleId,
              expectedModuleRevision,
            ) async {
              expect(
                entityKind,
                AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
              );
              expect(requestedGameRoot, gameRoot.path);
              expect(entityId, revision3NpcInspectionNpcId);
              expect(expectedEntityRevision, 2);
              expect(expectedModuleId, revision3NpcInspectionModuleId);
              expect(expectedModuleRevision, 3);
              return _npcManagedCompilerReceipt(
                projectJson: lease.canonicalProjectJson,
                head: lease.head,
                sourceSha256: fixture.sourceSha256,
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3NpcInspectionNpcId);
      final problemsTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-problemsChecks-$revision3NpcInspectionNpcId',
        ),
      );
      await tester.ensureVisible(problemsTab);
      await tester.tap(problemsTab);
      await tester.pumpAndSettle();
      final inspectNpc = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-inspect-npc_draft-$revision3NpcInspectionNpcId',
        ),
      );
      await _revealWorkbenchAction(tester, inspectNpc);
      expect(_workbenchActionTileWidget(tester, inspectNpc).enabled, isTrue);
      await _tapWorkbenchAction(tester, inspectNpc);
      await tester.pumpAndSettle();

      final panelFinder = find.byType(Revision3ManagedCompilerCheckPanel);
      expect(panelFinder, findsOneWidget);
      final panel = tester.widget<Revision3ManagedCompilerCheckPanel>(
        panelFinder,
      );
      final receipt = await panel.check();

      expect(receipt.acceptedAtExactCurrent, isTrue);
      expect(managed.managedCompilerCheckCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'selected Library Quest context edit reloads catalog and refreshes revision',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_quest_context_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final fixture = Revision3QuestOutlineFixture();
      var catalogLoads = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-context-authoring'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (lease) => Revision3QuestOutlineFixture(
          projectRevision: lease.projectRevision,
          questRevision: lease.projectRevision == fixture.projectRevision
              ? fixture.questRevision
              : fixture.questRevision + 1,
          moduleRevision: lease.projectRevision == fixture.projectRevision
              ? fixture.moduleRevision
              : fixture.moduleRevision + 1,
        ).contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: Revision3QuestOutlineFixture(
                    projectRevision: lease.projectRevision,
                    questRevision: questRevision,
                    moduleRevision: moduleRevision,
                  ).projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
        onQuestContextSeed:
            (
              lease,
              questId,
              questRevision,
              moduleId,
              moduleRevision,
              parentRuntime,
              giverRuntime,
            ) => AuthoringRevision3QuestContextSeed.forProject(
              currentProjectJson: fixture.projectJson,
              questId: questId,
              expectedQuestRevision: questRevision,
              expectedModuleId: moduleId,
              expectedModuleRevision: moduleRevision,
              expectedParentRuntimeClass: parentRuntime,
              expectedGiverRuntimeUniqueName: giverRuntime,
            ),
        onQuestContextPublish: (lease, requestedGameRoot, plan) {
          expect(requestedGameRoot, gameRoot.path);
          expect(plan.description, 'Find Homer and report back safely.');
          expect(plan.parentCatalogId, 'context-parent-current');
          expect(plan.giverCatalogId, 'context-giver-current');
          expect(plan.expectedParentAuthoringSelector, 'SwampCamp_SCChapter2');
          expect(plan.expectedGiverAuthoringSelector, 'OM_GRD_Asghan_263');
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestContextEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
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
        loadQuestCatalog: (requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          catalogLoads++;
          return _questContextCatalog(fixture);
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3QuestOutlineQuestId);
      final editStory = find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      );
      expect(editStory, findsOneWidget);
      await tester.ensureVisible(editStory);
      final journey = tester.widget<Revision3QuestJourneyView>(
        find.byType(Revision3QuestJourneyView),
      );
      expect(journey.onEditDescriptionConnections, isNotNull);
      expect(journey.editDisabledReason, isNull);
      expect(journey.editDescriptionConnectionsDisabledReason, isNull);
      final actionTooltip = find.ancestor(
        of: editStory,
        matching: find.byType(Tooltip),
      );
      final tooltipMessage = actionTooltip.evaluate().isEmpty
          ? null
          : tester.widget<Tooltip>(actionTooltip.first).message;
      expect(
        tester.widget<OutlinedButton>(editStory).onPressed,
        isNotNull,
        reason: 'unexpected Journey action gate: $tooltipMessage',
      );
      await tester.tap(editStory);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('revision3-quest-context-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('OM_GRD_'), findsNothing);
      await tester.enterText(
        find.byKey(const Key('revision3-quest-context-description')),
        'Find Homer and report back safely.',
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
      await tester.pumpAndSettle();

      expect(managed.questContextSeedCalls, 1);
      expect(catalogLoads, 2);
      expect(managed.questContextPublishCalls, 1);
      expect(managed.contentReadCalls, 8);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
      expect(
        find.textContaining('Quest description and connections saved'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-context-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'Content Quest continuation opens the exact canonical Story journey',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\content-to-story-journey'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (_) => fixture.contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: fixture.projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
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
      await _navigateManagedStory(tester);
      final references = find.byKey(
        const Key(
          'revision3-story-workbench-tab-references-'
          '$revision3QuestOutlineQuestId',
        ),
      );
      expect(references, findsOneWidget);
      await tester.ensureVisible(references);
      await tester.tap(references);
      await tester.pump();
      expect(tester.widget<ChoiceChip>(references).selected, isTrue);

      await _navigateManagedContent(tester);
      await tester.tap(
        find.byKey(
          const Key('revision3-content-entity-$revision3QuestOutlineQuestId'),
        ),
      );
      await tester.pumpAndSettle();

      final openStory = find.byKey(
        const Key('revision3-content-open-story-$revision3QuestOutlineQuestId'),
      );
      expect(openStory, findsOneWidget);
      expect(find.text('Open in Story'), findsOneWidget);
      await tester.ensureVisible(openStory);
      await tester.pump();
      await tester.tap(openStory);
      await tester.pumpAndSettle();

      final selectedQuest = find.byKey(
        const Key(
          'revision3-story-workspace-entity-$revision3QuestOutlineQuestId',
        ),
      );
      expect(selectedQuest, findsOneWidget);
      expect(tester.widget<ListTile>(selectedQuest).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-quest-journey-panel')),
        findsOneWidget,
      );
      final overview = find.byKey(
        const Key(
          'revision3-story-workbench-tab-overview-'
          '$revision3QuestOutlineQuestId',
        ),
      );
      expect(overview, findsOneWidget);
      expect(tester.widget<ChoiceChip>(overview).selected, isTrue);
      expect(find.text(revision3QuestOutlineQuestId), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Story Quest journey unifies exact behavior and editor handoffs',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-journey-authoring'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (_) => fixture.contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: fixture.projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
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
      await _navigateManagedStory(tester);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-quest-journey-panel')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-draft-setup')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-draft-setup-step-openingDialog')),
        findsOneWidget,
      );
      expect(find.text('Find Homer'), findsWidgets);
      expect(
        find.byKey(const Key('revision3-quest-journey-main-behavior')),
        findsOneWidget,
      );
      for (var index = 0; index < fixture.objectiveTitles.length; index++) {
        expect(
          find.byKey(Key('revision3-quest-journey-objective-$index')),
          findsOneWidget,
        );
      }
      expect(
        find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-quest-journey-edit-description-connections'),
        ),
        findsOneWidget,
        reason: 'only the catalog-bound context editor needs a game root',
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(
                const Key(
                  'revision3-quest-journey-edit-description-connections',
                ),
              ),
            )
            .onPressed,
        isNull,
      );
      final states = find.byKey(
        const Key('revision3-quest-journey-edit-states-transitions'),
      );
      expect(states, findsOneWidget);
      expect(find.text(revision3QuestOutlineQuestId), findsNothing);
      expect(find.text(revision3QuestOutlineModuleId), findsNothing);

      final overview = find.byKey(
        const Key(
          'revision3-story-workbench-section-overview-'
          '$revision3QuestOutlineQuestId',
        ),
      );
      await tester.scrollUntilVisible(
        states,
        240,
        scrollable: find
            .descendant(of: overview, matching: find.byType(Scrollable))
            .first,
      );
      await tester.pump();
      await tester.tap(states);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('revision3-quest-transitions-dialog')),
        findsOneWidget,
      );
      expect(managed.questTransitionsSeedCalls, 2);
      await tester.tap(
        find.byKey(const Key('revision3-quest-transitions-cancel')),
      );
      await tester.pumpAndSettle();

      final continueWithOpeningDialog = find.byKey(
        const Key('revision3-quest-draft-setup-recommended-dialog-voice'),
      );
      await tester.scrollUntilVisible(
        continueWithOpeningDialog,
        240,
        scrollable: find
            .descendant(of: overview, matching: find.byType(Scrollable))
            .first,
      );
      await tester.tap(continueWithOpeningDialog);
      await tester.pumpAndSettle();

      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-'
          '$revision3QuestOutlineQuestId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-quest-transcript-panel')),
        findsOneWidget,
      );
      final newLine = find.byKey(
        const Key('revision3-quest-transcript-new-line'),
      );
      expect(newLine, findsOneWidget);
      await tester.ensureVisible(newLine);
      expect(tester.widget<FilledButton>(newLine).onPressed, isNotNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'selected Library Quest behavior edit publishes without a game root',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = Revision3QuestOutlineFixture();
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\quest-transitions-authoring'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        contentIndexBuilder: (lease) => Revision3QuestOutlineFixture(
          projectRevision: lease.projectRevision,
          questRevision: lease.projectRevision == fixture.projectRevision
              ? fixture.questRevision
              : fixture.questRevision + 1,
          moduleRevision: lease.projectRevision == fixture.projectRevision
              ? fixture.moduleRevision
              : fixture.moduleRevision + 1,
        ).contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: fixture.projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
        onQuestTransitionsPublish: (lease, plan) {
          expect(plan.questId, revision3QuestOutlineQuestId);
          expect(plan.moduleId, revision3QuestOutlineModuleId);
          expect(
            plan.transitionPlan.transitions.any(
              (transition) => transition.effects.isNotEmpty,
            ),
            isTrue,
          );
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestTransitionsEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
            transitionPlanSeal: plan.transitionPlan.contentSeal,
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
      await _navigateManagedContent(tester);
      await _openStoryWorkbenchEntity(tester, revision3QuestOutlineQuestId);
      final editLogic = find.byKey(
        const Key('revision3-quest-journey-edit-states-transitions'),
      );
      expect(editLogic, findsOneWidget);
      await tester.ensureVisible(editLogic);
      expect(tester.widget<ButtonStyleButton>(editLogic).onPressed, isNotNull);
      await tester.tap(editLogic);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(
        find.byKey(const Key('revision3-quest-transitions-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(
          const Key('revision3-quest-transitions-sequential-template'),
        ),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-quest-transitions-save')),
      );
      await tester.pumpAndSettle();

      expect(managed.questTransitionsSeedCalls, 3);
      expect(managed.questTransitionsPublishCalls, 1);
      expect(managed.contentReadCalls, 8);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
      expect(
        find.textContaining('Quest states and transitions saved'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-quest-transitions-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'project Create mounts Story and opens the exact new NPC Dialog Voice',
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
        contentIndexBuilder: (lease) => lease.projectRevision == 7
            ? _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              )
            : _createdNpcContentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
                npcId: _homeCreatedNpcId,
                moduleId: _homeCreatedNpcModuleId,
                displayName: 'North Gate Guard',
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
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
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

      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
        reason: 'Story must still be lazy before the global Create command',
      );
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-project-create-npc')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('Asghan guard'), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'North Gate Guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();

      expect(catalogLoads, 2);
      expect(managed.npcPublishCalls, 1);
      expect(managed.contentReadCalls, greaterThanOrEqualTo(3));
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsOneWidget,
      );
      final selectedNpc = find.byKey(
        const Key('revision3-story-workspace-entity-$_homeCreatedNpcId'),
      );
      expect(selectedNpc, findsOneWidget);
      expect(tester.widget<ListTile>(selectedNpc).selected, isTrue);
      final dialogVoice = find.byKey(
        const Key(
          'revision3-story-workbench-tab-dialogVoice-$_homeCreatedNpcId',
        ),
      );
      expect(dialogVoice, findsOneWidget);
      expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
      expect(
        find.byKey(const Key('revision3-npc-dialog-voice-panel')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-npc-greeting-new-line')),
        findsOneWidget,
      );
      expect(
        find.textContaining(
          'could not be selected at its exact project revision',
        ),
        findsNothing,
      );
      expect(find.textContaining('Character draft saved'), findsOneWidget);
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(find.text('Build / Deploy'), findsNothing);
      for (final technicalIdentity in const <String>[
        _homeCreatedNpcId,
        _homeCreatedNpcModuleId,
        _homeCreatedNpcUniqueName,
        _homeCreatedNpcModuleNamespace,
      ]) {
        expect(find.text(technicalIdentity), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'cancelling project Create NPC stays on Home and publishes none',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_cancel_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\npc-authoring-cancel'),
        projectId: '20202020202020202020202020202020',
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-project-create-npc')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);

      await tester.tap(find.byKey(const Key('revision3-npc-cancel')));
      await tester.pumpAndSettle();

      expect(managed.npcPublishCalls, 0);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        7,
      );
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-home')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'project switch after NPC publication never follows the old Story link',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_project_switch_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final oldRoot = Directory(r'C:\mods\npc-project-switch-old');
      final newRoot = Directory(r'C:\mods\npc-project-switch-new');
      final switchCompleted = Completer<void>();
      late CurrentProjectCoordinator coordinator;
      final oldManaged = _FakeManagedLease(
        root: oldRoot,
        projectId: '21212121212121212121212121212121',
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onNpcPublish: (lease, _, _) {
          lease.projectRevision = 8;
          lease.head = _head(8);
          unawaited(
            (() async {
              try {
                await coordinator.closeCurrent();
                await coordinator.openManagedRevision3(newRoot);
                switchCompleted.complete();
              } catch (error, stackTrace) {
                switchCompleted.completeError(error, stackTrace);
              }
            })(),
          );
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: 8,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
          );
        },
      );
      final newManaged = _FakeManagedLease(
        root: newRoot,
        projectId: '22222222222222222222222222222222',
        projectRevision: 8,
        head: _head(8),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          projectName: 'Replacement project',
        ),
      );
      coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async =>
            root.path == oldRoot.path ? oldManaged : newManaged,
      );
      await coordinator.openManagedRevision3(oldRoot);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
        gamePath: gameRoot.path,
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-project-create-npc')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Old project guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pumpAndSettle();
      await switchCompleted.future;
      await tester.pumpAndSettle();

      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.root.path, newRoot.path);
      expect(current.projectId, newManaged.projectId);
      expect(current.projectRevision, 8);
      expect(oldManaged.npcPublishCalls, 1);
      expect(oldManaged.closeCalls, 1);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-home')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
      );
      expect(find.textContaining('Character draft saved'), findsNothing);
      expect(find.text('Old project guard'), findsNothing);
      for (final technicalIdentity in const <String>[
        _homeCreatedNpcId,
        _homeCreatedNpcModuleId,
      ]) {
        expect(find.text(technicalIdentity), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'published NPC head drift before dialog closes never opens Story',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_npc_head_drift_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '23232323232323232323232323232323';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\npc-head-drift'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onNpcPublish: (lease, _, _) {
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3NpcDraftPublication(
            projectId: lease.projectId,
            projectRevision: 8,
            head: lease.head,
            npcId: _homeCreatedNpcId,
            scriptModuleId: _homeCreatedNpcModuleId,
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
        loadNpcCatalog: (_) async => _npcCatalog(),
        chooseNpcArchetype: (_, _) async => 'g1r:npc:om_grd_asghan_263',
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('managed-project-create-npc')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-npc-display-name')),
        'Drifted guard',
      );
      await tester.tap(find.byKey(const Key('revision3-npc-submit')));
      await tester.pump();

      expect(managed.npcPublishCalls, 1);
      var current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.root.path, managed.root.path);
      expect(current.projectId, projectId);
      expect(current.projectRevision, 8);
      expect(current.head.canonicalJson, _head(8).canonicalJson);

      managed.head = _head(108);
      await coordinator.verifyCurrent();
      await tester.pumpAndSettle();

      current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.root.path, managed.root.path);
      expect(current.projectId, projectId);
      expect(current.projectRevision, 8);
      expect(current.head.canonicalJson, _head(108).canonicalJson);
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
      expect(
        find.byKey(const Key('revision3-project-workspace-page-home')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace-page-story')),
        findsNothing,
      );
      expect(find.text('Drifted guard'), findsNothing);
      expect(find.textContaining('Character draft saved'), findsNothing);
      expect(
        find.byKey(
          const Key('revision3-story-workspace-entity-$_homeCreatedNpcId'),
        ),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'empty managed project creates a dialog line and carries it into Voice',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_dialog_line_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '48484848484848484848484848484848';
      Revision3DialogLineEntryTechnicalPlan? publishedPlan;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\dialog-line-authoring'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) {
          final plan = publishedPlan;
          return plan == null
              ? _contentIndex(
                  projectId: lease.projectId,
                  revision: lease.projectRevision,
                )
              : _dialogLineContentIndex(
                  projectId: lease.projectId,
                  revision: lease.projectRevision,
                  plan: plan,
                );
        },
        onDialogLinePublish: (lease, plan) {
          expect(plan.lineDisplayName, 'Mine entrance warning');
          expect(plan.speakerHint, 'Asghan');
          expect(plan.locale, 'de');
          final localization =
              plan.localization
                  as AuthoringRevision3DialogLocalizationCreateIntentV1;
          expect(localization.displayName, 'Mine entrance warning text');
          expect(localization.texts, <String, String>{
            'de': 'Halt! Niemand betritt die Mine.',
          });
          expect(plan.voiceSlot?.locale, 'de');
          expect(plan.voiceSlot?.displayName, 'Mine entrance warning de Voice');
          publishedPlan = plan;
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3DialogLineEntryPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            lineId: plan.lineId,
            localizationId: localization.localizationId,
            localizationAction:
                AuthoringRevision3DialogLocalizationAction.created,
            voiceSlotId: plan.voiceSlot?.slotId,
            locale: plan.locale,
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

      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      final createButton = find.byKey(
        const Key('revision3-localization-new-line'),
      );
      expect(createButton, findsOneWidget);
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-new-line'),
        enabled: true,
      );
      final l10n = AppLocalizations.of(tester.element(createButton));

      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-new-line'),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsOneWidget,
      );
      expect(find.text(l10n.managedDialogLineBoundary), findsOneWidget);

      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        'Mine entrance warning',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-speaker')),
        'Asghan',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-locale')),
        'de',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-text')),
        'Halt! Niemand betritt die Mine.',
      );
      final submit = find.byKey(const Key('revision3-dialog-line-submit'));
      await tester.ensureVisible(submit);
      await tester.tap(submit);
      await tester.pumpAndSettle();

      expect(managed.dialogLinePublishCalls, 1);
      expect(publishedPlan, isNotNull);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(
        find.byKey(const Key('revision3-dialog-line-success')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedActionNewDialogLineSaved(8)),
        findsOneWidget,
      );
      expect(find.text(l10n.managedDialogLineBoundary), findsOneWidget);

      await tester.tap(
        find.byKey(const Key('revision3-dialog-line-open-voice')),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-voice-wizard')), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-fixed-context')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('revision3-voice-wizard')),
          matching: find.text('Asghan — Mine entrance warning'),
        ),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-line-search')),
        findsNothing,
      );
      expect(find.byKey(const Key('revision3-voice-locale')), findsNothing);
      expect(managed.voicePublishCalls, 0);

      await tester.tap(find.byKey(const Key('revision3-voice-cancel')));
      await tester.pumpAndSettle();
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: true,
      );
      expect(
        find.text(l10n.managedActionNewDialogLineSaved(8)),
        findsOneWidget,
      );
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets(
    'Dialog and Voice reuses exact project text only after bounded preview verification',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const projectId = '49494949494949494949494949494949';
      const localizationId = '59595959595959595959595959595959';
      const localizationRevision = 4;
      const locId = 'GORE_SHARED_MINE_WARNING';
      const displayName = 'Shared mine warning text';
      const previewText = 'Halt! Dieser Weg bleibt gesperrt.';
      Revision3DialogLineEntryTechnicalPlan? publishedPlan;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\dialog-line-reuse'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _dialogReuseContentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          localizationId: localizationId,
          localizationRevision: localizationRevision,
          locId: locId,
          displayName: displayName,
          publishedPlan: publishedPlan,
        ),
        onDialogLocalizationRead:
            (lease, requestedId, requestedRevision, requestedLocId) {
              expect(lease.projectRevision, 7);
              expect(lease.head.canonicalJson, _head(7).canonicalJson);
              expect(requestedId, localizationId);
              expect(requestedRevision, localizationRevision);
              expect(requestedLocId, locId);
              return _dialogLocalizationReadResult(
                lease: lease,
                localizationId: requestedId,
                localizationRevision: requestedRevision,
                locId: requestedLocId,
                nonemptyPreview: previewText,
              );
            },
        onDialogLinePublish: (lease, plan) {
          expect(lease.projectRevision, 7);
          expect(lease.head.canonicalJson, _head(7).canonicalJson);
          expect(plan.lineDisplayName, 'Reused mine warning');
          expect(plan.speakerHint, 'Asghan');
          expect(plan.locale, 'de');
          final reuse =
              plan.localization
                  as AuthoringRevision3DialogLocalizationReuseExactIntentV1;
          expect(reuse.localizationId, localizationId);
          expect(reuse.expectedLocalizationRevision, localizationRevision);
          expect(reuse.expectedLocId, locId);
          expect(plan.voiceSlot?.locale, 'de');
          publishedPlan = plan;
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3DialogLineEntryPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            lineId: plan.lineId,
            localizationId: reuse.localizationId,
            localizationAction:
                AuthoringRevision3DialogLocalizationAction.reusedExact,
            voiceSlotId: plan.voiceSlot?.slotId,
            locale: plan.locale,
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

      void expectTechnicalIdentityHidden() {
        expect(
          find.textContaining(
            localizationId,
            findRichText: true,
            skipOffstage: false,
          ),
          findsNothing,
        );
        expect(
          find.textContaining(locId, findRichText: true, skipOffstage: false),
          findsNothing,
        );
        final plan = publishedPlan;
        if (plan != null) {
          expect(
            find.textContaining(
              plan.lineId,
              findRichText: true,
              skipOffstage: false,
            ),
            findsNothing,
          );
          expect(
            find.textContaining(
              plan.lineAuthoredIdentity,
              findRichText: true,
              skipOffstage: false,
            ),
            findsNothing,
          );
          final slot = plan.voiceSlot;
          if (slot != null) {
            expect(
              find.textContaining(
                slot.slotId,
                findRichText: true,
                skipOffstage: false,
              ),
              findsNothing,
            );
          }
        }
      }

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      expectTechnicalIdentityHidden();

      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      final createButton = find.byKey(
        const Key('revision3-localization-new-line'),
      );
      final l10n = AppLocalizations.of(tester.element(createButton));
      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-new-line'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.managedDialogLineReuseMode));
      await tester.pump();

      final reuseCandidate = find.descendant(
        of: find.byKey(const Key('revision3-dialog-line-modal')),
        matching: find.text(displayName),
      );
      expect(reuseCandidate, findsOneWidget);
      expectTechnicalIdentityHidden();
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        'Reused mine warning',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-speaker')),
        'Asghan',
      );
      await tester.tap(reuseCandidate);
      await tester.pumpAndSettle();

      expect(managed.dialogLocalizationReadCalls, 1);
      expect(
        find.byKey(const Key('revision3-dialog-line-reuse-preview')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dialog-line-preview-text')),
        findsOneWidget,
      );
      expect(find.text(previewText), findsOneWidget);
      await tester.drag(
        find.byKey(const Key('revision3-dialog-line-editor')),
        const Offset(0, -280),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dialog-line-locale-de')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dialog-line-locale-en')),
        findsNothing,
      );
      expectTechnicalIdentityHidden();

      final submit = find.byKey(const Key('revision3-dialog-line-submit'));
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-locale')),
        'en',
      );
      await tester.pump();
      expect(tester.widget<FilledButton>(submit).onPressed, isNull);
      expect(
        find.byKey(const Key('revision3-dialog-line-reuse-preview')),
        findsNothing,
      );
      await tester.tap(
        find.byKey(const Key('revision3-dialog-line-locale-de')),
      );
      await tester.pump();
      expect(tester.widget<FilledButton>(submit).onPressed, isNotNull);

      await tester.ensureVisible(submit);
      await tester.tap(submit);
      await tester.pumpAndSettle();

      expect(managed.dialogLocalizationReadCalls, 2);
      expect(managed.dialogLinePublishCalls, 1);
      expect(publishedPlan, isNotNull);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(
        find.byKey(const Key('revision3-dialog-line-success')),
        findsOneWidget,
      );
      expectTechnicalIdentityHidden();

      await tester.tap(find.byKey(const Key('revision3-dialog-line-done')));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.managedActionNewDialogLineSaved(8)),
        findsOneWidget,
      );
      expectTechnicalIdentityHidden();
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

      expect(find.byKey(const Key('managed-home-story')), findsOneWidget);
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: true,
      );

      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-add-voice'),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-voice-wizard')), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('revision3-voice-line-search')),
        'GRD_263_ASGHAN',
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.descendant(
          of: find.byKey(const Key('revision3-voice-line-results')),
          matching: find.text('Asghan — Mine entrance question'),
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
      await _expandManagedTechnicalDetails(tester);
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
    'configured Voice actions stay visible with localized dialog prerequisite',
    (tester) async {
      await _setNarrowShortTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_prerequisite_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-prerequisite'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
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
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));

      final sectionAction = find.byKey(
        const Key('revision3-localization-add-voice'),
      );
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );
      final sectionPrerequisite = AppLocalizations.of(
        tester.element(sectionAction),
      ).managedActionAddVoiceTakeRequiresDialogLine;
      expect(find.text(sectionPrerequisite), findsOneWidget);
      expect(managed.voicePublishCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Home opens Voice work list first and keeps missing-game recording visibly disabled',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-production-work-list'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _voiceLocalizationWorkspaceIndex(revision: lease.projectRevision),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, locId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: localizationId,
                  localizationRevision: localizationRevision,
                  locId: locId,
                  lineId: revision3VoiceContentLineId,
                  lineDisplayName: 'Mine entrance question',
                  speaker: 'Asghan',
                  voiceSlotLocales: const <String>{'de'},
                ),
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
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));

      expect(
        find.byKey(const Key('revision3-localization-voice-mode')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-voice-work-list')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-queue')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-text-browser')),
        findsNothing,
      );

      const itemKey = 'voice:$revision3VoiceContentLineId:de';
      final action = find.byKey(
        const ValueKey('revision3-voice-production-queue-action-$itemKey'),
      );
      await _scrollManagedVoiceQueueUntilVisible(tester, action);
      expect(tester.widget<FilledButton>(action).onPressed, isNull);
      final l10n = AppLocalizations.of(tester.element(action));
      final reason = l10n.managedDashboardMissingGameDescription;
      final disabledReason = find.byKey(
        const ValueKey('revision3-voice-production-queue-disabled-$itemKey'),
      );
      expect(disabledReason, findsOneWidget);
      expect(tester.widget<Text>(disabledReason).data, reason);

      await _switchManagedLocalizationVoiceToProjectTexts(tester);
      expect(
        find.byKey(const Key('revision3-localization-text-browser')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-text-editor')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-voice-work-list')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('complete Voice work-list item opens Validate and Test checks', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\voice-production-review-checks'),
      projectId: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      contentIndexBuilder: (lease) => _voiceLocalizationWorkspaceIndex(
        revision: lease.projectRevision,
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
        existingSlotTargetResolution: 'resolved',
      ),
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
    await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));

    const itemKey = 'voice:$revision3VoiceContentLineId:de';
    final action = find.byKey(
      const ValueKey('revision3-voice-production-queue-action-$itemKey'),
    );
    await _scrollManagedVoiceQueueUntilVisible(tester, action);
    expect(tester.widget<FilledButton>(action).onPressed, isNotNull);
    expect(
      find.descendant(of: action, matching: find.text('Review checks')),
      findsOneWidget,
    );
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-workspace-page-validate-test')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-problems-view')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-localization-voice-work-list')),
      findsNothing,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('exact DialogLine enables Voice actions in their workspace', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final gameRoot = Directory.systemTemp.createTempSync(
      'gore_r3_voice_dialog_line_game_',
    );
    Directory(p.join(gameRoot.path, 'G1R')).createSync();
    addTearDown(() {
      if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
    });
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\voice-dialog-line'),
      projectId: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      contentIndexBuilder: (lease) =>
          _voiceLocalizationWorkspaceIndex(revision: lease.projectRevision),
      onDialogLocalizationEditSeed:
          (lease, localizationId, localizationRevision, locId) =>
              _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: locId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance question',
                speaker: 'Asghan',
                voiceSlotLocales: const <String>{'de'},
              ),
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

    await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
    await _switchManagedLocalizationVoiceToProjectTexts(tester);

    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-add-voice'),
      enabled: true,
    );
    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-manage-voice'),
      enabled: true,
    );
    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-resolve-voice'),
      enabled: true,
    );
    expect(
      find.byKey(const Key('revision3-voice-production-intact')),
      findsOneWidget,
    );
    expect(find.text('0 takes'), findsOneWidget);
    expect(find.text('Target: Unresolved'), findsOneWidget);

    final contextualManage = find.byKey(
      const Key('revision3-voice-production-manage'),
    );
    await _scrollManagedEditorUntilVisible(tester, contextualManage);
    await tester.tap(contextualManage);
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('voice-selection-fixed-context')),
      findsOneWidget,
    );
    expect(find.text('Voice language: de'), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-line-search')), findsNothing);
    await tester.tap(find.byKey(const Key('voice-selection-cancel')));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'dirty contextual Manage saves, rebinds the production host, and continues with exact current context',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const changedText = 'Saved before opening the current Voice setup';
      const locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
      var localizationRevision = 0;
      var savedEnglishText = 'Stop right there!';
      final seedProjectRevisions = <int>[];
      final seedLocalizationRevisions = <int>[];
      Revision3DialogLocalizationEditTechnicalPlan? publishedPlan;
      Revision3DialogLocalizationEditPublication? returnedPublication;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-context-save-and-continue'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _voiceLocalizationWorkspaceIndex(
          revision: lease.projectRevision,
          localizationRevision: localizationRevision,
        ),
        onDialogLocalizationEditSeed:
            (lease, localizationId, requestedRevision, requestedLocId) {
              seedProjectRevisions.add(lease.projectRevision);
              seedLocalizationRevisions.add(requestedRevision);
              expect(localizationId, revision3VoiceContentLocalizationId);
              expect(requestedRevision, localizationRevision);
              expect(requestedLocId, locId);
              return _dialogLocalizationEditSeed(
                lease: lease,
                localizationId: localizationId,
                localizationRevision: requestedRevision,
                locId: requestedLocId,
                lineId: revision3VoiceContentLineId,
                lineDisplayName: 'Mine entrance question',
                speaker: 'Asghan',
                voiceSlotLocales: const <String>{'de'},
                englishText: savedEnglishText,
              );
            },
        onDialogLocalizationEditPublish: (lease, plan) {
          expect(plan.expectedHead.canonicalJson, _head(7).canonicalJson);
          expect(plan.localizationId, revision3VoiceContentLocalizationId);
          expect(plan.expectedLocalizationRevision, 0);
          expect(plan.expectedLocId, locId);
          expect(plan.texts, const <String, String>{
            'de': 'Bleib stehen!',
            'en': changedText,
          });
          publishedPlan = plan;
          savedEnglishText = plan.texts['en']!;
          localizationRevision = plan.expectedLocalizationRevision + 1;
          lease.projectRevision++;
          lease.head = _head(lease.projectRevision);
          return returnedPublication =
              Revision3DialogLocalizationEditPublication(
                projectId: lease.projectId,
                projectRevision: lease.projectRevision,
                localizationId: plan.localizationId,
                localizationRevision: localizationRevision,
                addedLocales: const <String>[],
                removedLocales: const <String>[],
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
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);

      final textField = find.byKey(const Key('revision3-localization-text-en'));
      final l10n = AppLocalizations.of(tester.element(textField));
      await tester.enterText(textField, changedText);
      await tester.pump();
      final contextualManage = find.byKey(
        const Key('revision3-voice-production-manage'),
      );
      await _scrollManagedEditorUntilVisible(tester, contextualManage);
      await tester.tap(contextualManage);
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.managedLocalizationVoiceUnsavedTitle),
        findsOneWidget,
      );
      expect(
        find.text(l10n.managedLocalizationSaveAndContinue),
        findsOneWidget,
      );
      await tester.tap(find.text(l10n.managedLocalizationSaveAndContinue));
      await tester.pumpAndSettle();

      expect(managed.dialogLocalizationEditPublishCalls, 1);
      expect(publishedPlan, isNotNull);
      expect(returnedPublication?.projectRevision, 8);
      expect(returnedPublication?.localizationRevision, 1);
      expect(savedEnglishText, changedText);
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, 8);
      expect(current.head.canonicalJson, _head(8).canonicalJson);
      final reboundWorkspace = tester
          .widget<Revision3LocalizationVoiceWorkspace>(
            find.byType(Revision3LocalizationVoiceWorkspace),
          );
      expect(reboundWorkspace.projectRevision, 8);
      expect(
        reboundWorkspace.projectCheckpointIdentity,
        _head(8).canonicalJson,
      );
      expect(reboundWorkspace.onManageVoiceTakesFor, isNotNull);
      expect(managed.dialogLocalizationEditSeedCalls, greaterThanOrEqualTo(2));
      expect(seedProjectRevisions, containsAllInOrder(<int>[7, 8]));
      expect(seedLocalizationRevisions, containsAllInOrder(<int>[0, 1]));
      expect(find.textContaining(l10n.managedLocalizationStale), findsNothing);
      expect(
        find.text(l10n.managedLocalizationVoiceActionFailed),
        findsNothing,
      );

      final dialog = find.byType(Revision3VoiceTakeSelectionDialog);
      expect(dialog, findsOneWidget);
      final currentDialog = tester.widget<Revision3VoiceTakeSelectionDialog>(
        dialog,
      );
      expect(currentDialog.fixedContext, isTrue);
      expect(currentDialog.initialLineId, revision3VoiceContentLineId);
      expect(currentDialog.initialLocale, 'de');
      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.textContaining('Asghan'), findsWidgets);
      expect(find.textContaining('Mine entrance question'), findsWidgets);
      expect(find.text('Voice-Sprache: de'), findsOneWidget);
      expect(
        find.byKey(const Key('voice-selection-line-search')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );
      expect(tester.widget<TextField>(textField).controller!.text, changedText);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNull,
      );
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      expect(find.text(revision3VoiceContentSlotId), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'German Localization and Voice opens all Voice dialogs with distinct global actions',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_dialogs_de_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-dialogs-de'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _voiceLocalizationWorkspaceIndex(revision: lease.projectRevision),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, locId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: localizationId,
                  localizationRevision: localizationRevision,
                  locId: locId,
                  lineId: revision3VoiceContentLineId,
                  lineDisplayName: 'Mine entrance question',
                  speaker: 'Asghan',
                  voiceSlotLocales: const <String>{'de'},
                ),
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
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);

      final l10n = AppLocalizations.of(
        tester.element(
          find.byKey(const Key('revision3-localization-voice-workspace')),
        ),
      );
      const contextualCopy = Revision3VoiceProductionCardCopy.german;
      final actionLabels =
          <
            ({
              Key globalKey,
              String globalLabel,
              Key contextualKey,
              String contextualLabel,
            })
          >[
            (
              globalKey: const Key('revision3-localization-add-voice'),
              globalLabel: l10n.managedLocalizationGlobalAddVoice,
              contextualKey: const Key('revision3-voice-production-add'),
              contextualLabel: contextualCopy.addTakeLabel,
            ),
            (
              globalKey: const Key('revision3-localization-manage-voice'),
              globalLabel: l10n.managedLocalizationGlobalManageVoice,
              contextualKey: const Key('revision3-voice-production-manage'),
              contextualLabel: contextualCopy.manageTakesLabel,
            ),
            (
              globalKey: const Key('revision3-localization-resolve-voice'),
              globalLabel: l10n.managedLocalizationGlobalResolveVoice,
              contextualKey: const Key('revision3-voice-production-resolve'),
              contextualLabel: contextualCopy.resolveTargetLabel,
            ),
          ];
      for (final labels in actionLabels) {
        final globalAction = find.byKey(labels.globalKey);
        final contextualAction = find.byKey(labels.contextualKey);
        expect(globalAction, findsOneWidget);
        expect(contextualAction, findsOneWidget);
        expect(labels.globalLabel, isNot(labels.contextualLabel));
        expect(
          find.descendant(
            of: globalAction,
            matching: find.text(labels.globalLabel),
          ),
          findsOneWidget,
        );
        expect(
          find.descendant(
            of: contextualAction,
            matching: find.text(labels.contextualLabel),
          ),
          findsOneWidget,
        );
      }

      Future<void> openGlobalVoiceAction(Key key) async {
        final action = find.byKey(key);
        final button = find.descendant(
          of: action,
          matching: find.byType(OutlinedButton),
        );
        expect(button, findsOneWidget);
        expect(tester.widget<OutlinedButton>(button).onPressed, isNotNull);
        await tester.ensureVisible(button);
        await tester.tap(button);
        await tester.pumpAndSettle();
      }

      await openGlobalVoiceAction(
        const Key('revision3-localization-add-voice'),
      );
      final addDialog = find.byType(Revision3VoiceTakeDialog);
      expect(addDialog, findsOneWidget);
      expect(
        tester.widget<Revision3VoiceTakeDialog>(addDialog).copy.title,
        'Voice-Take hinzufügen',
      );
      expect(
        find.descendant(
          of: addDialog,
          matching: find.text('Nur im Projekt gespeichert'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-line-search')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('revision3-voice-cancel')));
      await tester.pumpAndSettle();

      await openGlobalVoiceAction(
        const Key('revision3-localization-manage-voice'),
      );
      final manageDialog = find.byType(Revision3VoiceTakeSelectionDialog);
      expect(manageDialog, findsOneWidget);
      expect(
        tester
            .widget<Revision3VoiceTakeSelectionDialog>(manageDialog)
            .copy
            .title,
        'Voice-Takes verwalten',
      );
      expect(
        find.descendant(
          of: manageDialog,
          matching: find.text('Dialogzeile finden'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('voice-selection-line-search')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('voice-selection-cancel')));
      await tester.pumpAndSettle();

      await openGlobalVoiceAction(
        const Key('revision3-localization-resolve-voice'),
      );
      final targetDialog = find.byType(Revision3VoiceTargetDialog);
      expect(targetDialog, findsOneWidget);
      expect(
        tester.widget<Revision3VoiceTargetDialog>(targetDialog).copy.title,
        'Installiertes Voice-Ziel auflösen',
      );
      expect(
        find.descendant(
          of: targetDialog,
          matching: find.text('Keine Bereitstellung'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-target-line-search')),
        findsOneWidget,
      );
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      expect(find.text(revision3VoiceContentSlotId), findsNothing);
      await tester.tap(find.byKey(const Key('revision3-voice-target-cancel')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Spanish Localization and Voice keeps translated global Voice actions',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_dialogs_es_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-dialogs-es'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) =>
            _voiceLocalizationWorkspaceIndex(revision: lease.projectRevision),
        onDialogLocalizationEditSeed:
            (lease, localizationId, localizationRevision, locId) =>
                _dialogLocalizationEditSeed(
                  lease: lease,
                  localizationId: localizationId,
                  localizationRevision: localizationRevision,
                  locId: locId,
                  lineId: revision3VoiceContentLineId,
                  lineDisplayName: 'Mine entrance question',
                  speaker: 'Asghan',
                  voiceSlotLocales: const <String>{'de'},
                ),
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
      container.read(localeProvider.notifier).setLocale('es');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);

      const actions = <(Key, String)>[
        (Key('revision3-localization-add-voice'), 'Añadir toma de voz'),
        (Key('revision3-localization-manage-voice'), 'Gestionar tomas de voz'),
        (
          Key('revision3-localization-resolve-voice'),
          'Resolver destino de voz',
        ),
      ];
      for (final (key, label) in actions) {
        expect(
          find.descendant(of: find.byKey(key), matching: find.text(label)),
          findsOneWidget,
        );
      }
      expect(find.text('Add take for any line'), findsNothing);
      expect(find.text('Manage takes for any line'), findsNothing);
      expect(find.text('Resolve target for any line'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'unresolved DialogLine localization keeps catalog-dependent Voice actions disabled',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_unresolved_localization_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-unresolved-localization'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _voiceIndexWithUnresolvedLocalization(
          revision: lease.projectRevision,
        ),
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

      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      for (final key in const <Key>[
        Key('revision3-localization-add-voice'),
        Key('revision3-localization-manage-voice'),
        Key('revision3-localization-resolve-voice'),
      ]) {
        _expectLocalizationVoiceAction(tester, key: key, enabled: false);
      }
      expect(find.byType(Revision3VoiceTakeDialog), findsNothing);
      expect(find.byType(Revision3VoiceTakeSelectionDialog), findsNothing);
      expect(find.byType(Revision3VoiceTargetDialog), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'late DialogLine index cannot enable Voice add for a newer empty revision',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_voice_stale_gate_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final staleGateLoad = Completer<Revision3ContentIndex>();
      final freshGateLoad = Completer<Revision3ContentIndex>();
      var contentRead = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-stale-gate'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onContentIndexRead: (lease) {
          contentRead++;
          return switch (contentRead) {
            1 => Future.value(
              _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              ),
            ),
            2 => staleGateLoad.future,
            3 => Future.value(
              _contentIndex(projectId: lease.projectId, revision: 8),
            ),
            4 => freshGateLoad.future,
            _ => Future.value(
              _contentIndex(
                projectId: lease.projectId,
                revision: lease.projectRevision,
              ),
            ),
          };
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
      await _navigateManagedLocalizationVoice(tester, settle: false);
      await tester.pump();
      expect(contentRead, 2);
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );

      managed.projectRevision = 8;
      managed.head = _head(8);
      (coordinator as dynamic).state = ManagedRevision3CurrentProjectState(
        root: managed.root,
        projectId: managed.projectId,
        projectRevision: managed.projectRevision,
        head: managed.head,
        requiresReopen: false,
      );
      await tester.pump();

      staleGateLoad.complete(revision3VoiceContentIndexFixture(revision: 7));
      for (var index = 0; index < 10 && contentRead < 4; index++) {
        await tester.pump();
      }
      expect(contentRead, 4);
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );

      freshGateLoad.complete(
        _contentIndex(projectId: managed.projectId, revision: 8),
      );
      await tester.pumpAndSettle();
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-add-voice'),
        enabled: false,
      );

      await _navigateManagedHome(tester);
      final dialogVoiceTask = find.byKey(
        const Key('managed-home-dialog-voice'),
      );
      expect(dialogVoiceTask, findsOneWidget);
      expect(tester.widget<ListTile>(dialogVoiceTask).onTap, isNotNull);
      expect(find.byKey(const Key('managed-add-voice-take')), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'managed Voice media QA is wired through Dialog and Voice without a game root or write',
    (tester) async {
      await _setDesktopTestSurface(tester);
      tester.view.physicalSize = const Size(1600, 1200);
      final managed = _FakeVoiceMediaQaManagedLease(
        root: Directory(r'C:\mods\voice-media-qa'),
        projectId: revision3VoicePreviewProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (_) =>
            revision3VoicePreviewContentIndex(revision: 7),
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
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-manage-voice'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();

      final check = find.byKey(const Key('voice-media-qa-start-0'));
      await tester.ensureVisible(check);
      await tester.tap(check);
      await tester.pumpAndSettle();

      expect(managed.voiceMediaQaCalls, 1);
      expect(find.byKey(const Key('voice-media-qa-result-0')), findsOneWidget);
      expect(find.textContaining('0.08 s'), findsOneWidget);
      expect(find.textContaining('fully decoded'), findsOneWidget);
      expect(find.textContaining('not audio quality'), findsOneWidget);
      expect(find.textContaining('in-game playback'), findsOneWidget);
      expect(managed.projectRevision, 7);
      expect(managed.requiresReopen, isFalse);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        7,
      );
    },
  );

  testWidgets(
    'managed Voice selection works without a game root and retains Library selection',
    (tester) async {
      await _setDesktopTestSurface(tester);
      tester.view.physicalSize = const Size(1600, 1200);
      const firstTakeId = '55000000000000000000000000000000';
      const secondTakeId = '55000000000000000000000000000001';
      var currentIndex = _voiceSelectionIndex(
        revision: 7,
        selectedTakeId: firstTakeId,
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-selection'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (_) => currentIndex,
        onVoiceSelectionPublish: (lease, plan) {
          expect(plan.lineId, revision3VoiceContentLineId);
          expect(plan.slotId, revision3VoiceContentSlotId);
          expect(plan.locale, 'de');
          expect(plan.expectedSelectedTakeId, firstTakeId);
          expect(plan.selectedTakeId, secondTakeId);
          lease.projectRevision = 8;
          lease.head = _head(8);
          currentIndex = _voiceSelectionIndex(
            revision: 8,
            selectedTakeId: secondTakeId,
            slotRevision: plan.expectedSlotRevision + 1,
          );
          return Revision3VoiceTakeSelectionPublication(
            projectId: lease.projectId,
            projectRevision: 8,
            lineId: plan.lineId,
            slotId: plan.slotId,
            slotRevision: plan.expectedSlotRevision + 1,
            locale: plan.locale,
            locId: plan.locId,
            previousSelectedTakeId: plan.expectedSelectedTakeId,
            selectedTakeId: plan.selectedTakeId,
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

      final missingGame = AppLocalizations.of(
        tester.element(find.byKey(const Key('managed-home-dialog-voice'))),
      ).managedDashboardMissingGameDescription;
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-manage-voice'),
        enabled: true,
      );
      for (final key in const <Key>[
        Key('revision3-localization-add-voice'),
        Key('revision3-localization-resolve-voice'),
      ]) {
        _expectLocalizationVoiceAction(tester, key: key, enabled: false);
      }
      expect(find.text(missingGame), findsWidgets);
      await _navigateManagedContent(tester);
      final libraryLine = find.byKey(
        const Key('revision3-content-entity-$revision3VoiceContentLineId'),
      );
      await tester.ensureVisible(libraryLine);
      await tester.tap(libraryLine);
      await tester.pump();
      expect(tester.widget<ListTile>(libraryLine).selected, isTrue);

      await _navigateManagedHome(tester);
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-manage-voice'),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.ensureVisible(
        find.byKey(const Key('voice-selection-take-1')),
      );
      await tester.tap(find.byKey(const Key('voice-selection-take-1')));
      await tester.pump();
      await tester.ensureVisible(find.byKey(const Key('voice-selection-save')));
      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(managed.voiceSelectionPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
      await _expandManagedTechnicalDetails(tester);
      expect(find.text('8'), findsWidgets);
      expect(
        find.textContaining(
          'Approved Voice take selected in project revision 8',
        ),
        findsOneWidget,
      );
      await _navigateManagedContent(tester);
      final reloadedLibraryLine = find.byKey(
        const Key('revision3-content-entity-$revision3VoiceContentLineId'),
      );
      expect(
        tester.widget<ListTile>(reloadedLibraryLine).selected,
        isTrue,
        reason:
            'the same visible line remains selected after exact-head reload',
      );
      expect(managed.contentReadCalls, greaterThanOrEqualTo(6));
    },
  );

  testWidgets(
    'Voice status then selection in one dialog uses the latest coordinator head',
    (tester) async {
      await _setDesktopTestSurface(tester);
      tester.view.physicalSize = const Size(1600, 1200);
      const firstTakeId = '55000000000000000000000000000000';
      const secondTakeId = '55000000000000000000000000000001';
      var currentIndex = _voiceSelectionIndex(
        revision: 7,
        selectedTakeId: firstTakeId,
        alternateStatus: 'recorded',
      );
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\voice-status-dynamic-head'),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (_) => currentIndex,
        onVoiceStatusPublish: (lease, plan) {
          expect(plan.takeId, secondTakeId);
          expect(
            plan.expectedStatus,
            AuthoringRevision3VoiceTakeStatus.recorded,
          );
          expect(
            plan.desiredStatus,
            AuthoringRevision3VoiceTakeStatus.approved,
          );
          expect(lease.projectRevision, 7);
          expect(lease.head.canonicalJson, _head(7).canonicalJson);
          lease.projectRevision = 8;
          lease.head = _head(8);
          currentIndex = _voiceSelectionIndex(
            revision: 8,
            selectedTakeId: firstTakeId,
            alternateStatus: 'approved',
            alternateRevision: 1,
          );
          return Revision3VoiceTakeStatusPublication(
            projectId: lease.projectId,
            projectRevision: 8,
            lineId: plan.lineId,
            localizationId: plan.localizationId,
            slotId: plan.slotId,
            slotRevision: plan.expectedSlotRevision,
            locale: plan.locale,
            locId: plan.locId,
            takeId: plan.takeId,
            takeRevision: plan.expectedTakeRevision + 1,
            previousStatus: plan.expectedStatus,
            status: plan.desiredStatus,
          );
        },
        onVoiceSelectionPublish: (lease, plan) {
          expect(
            lease.head.canonicalJson,
            _head(8).canonicalJson,
            reason: 'selection must run from the status publication head',
          );
          expect(lease.projectRevision, 8);
          expect(plan.expectedSelectedTakeId, firstTakeId);
          expect(plan.selectedTakeId, secondTakeId);
          lease.projectRevision = 9;
          lease.head = _head(9);
          currentIndex = _voiceSelectionIndex(
            revision: 9,
            selectedTakeId: secondTakeId,
            slotRevision: plan.expectedSlotRevision + 1,
            selectedRevision: 1,
          );
          return Revision3VoiceTakeSelectionPublication(
            projectId: lease.projectId,
            projectRevision: 9,
            lineId: plan.lineId,
            slotId: plan.slotId,
            slotRevision: plan.expectedSlotRevision + 1,
            locale: plan.locale,
            locId: plan.locId,
            previousSelectedTakeId: plan.expectedSelectedTakeId,
            selectedTakeId: plan.selectedTakeId,
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
      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-manage-voice'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.ensureVisible(
        find.byKey(const Key('voice-status-change-1')),
      );
      await tester.tap(find.byKey(const Key('voice-status-change-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
      await tester.pumpAndSettle();

      expect(managed.voiceStatusPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
      expect(
        tester
            .widget<RadioListTile<String>>(
              find.byKey(const Key('voice-selection-take-1')),
            )
            .enabled,
        isTrue,
      );

      await tester.tap(find.byKey(const Key('voice-selection-take-1')));
      await tester.pump();
      await tester.ensureVisible(find.byKey(const Key('voice-selection-save')));
      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(managed.voiceStatusPublishCalls, 1);
      expect(managed.voiceSelectionPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        9,
      );
      expect(
        find.textContaining(
          'Approved Voice take selected in project revision 9',
        ),
        findsOneWidget,
      );
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
        onVoicePlan: _readyVoicePlan,
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

      await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
      _expectLocalizationVoiceAction(
        tester,
        key: const Key('revision3-localization-resolve-voice'),
        enabled: true,
      );

      await _tapLocalizationVoiceAction(
        tester,
        const Key('revision3-localization-resolve-voice'),
      );
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
        find.descendant(
          of: find.byKey(const Key('revision3-voice-target-line-results')),
          matching: find.text('Asghan — Mine entrance question'),
        ),
      );
      await tester.pumpAndSettle();
      final targetSubmit = find.byKey(
        const Key('revision3-voice-target-submit'),
      );
      await tester.ensureVisible(targetSubmit);
      await tester.tap(targetSubmit);
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

      await _navigateManagedHome(tester);
      await _tapManagedHomeTask(tester, const Key('managed-home-build'));
      _expectManagedSectionAction(
        tester,
        sectionId: 'build-release',
        actionId: 'build-voice-bundle',
        enabled: true,
      );
      final buildBundle = _managedSectionAction(
        sectionId: 'build-release',
        actionId: 'build-voice-bundle',
      );
      await tester.ensureVisible(buildBundle);
      await tester.tap(buildBundle);
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

    expect(
      find.byKey(const Key('revision3-project-dashboard-missing-game')),
      findsOneWidget,
    );
    await _tapManagedHomeTask(tester, const Key('managed-home-dialog-voice'));
    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-add-voice'),
      enabled: false,
    );
    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-manage-voice'),
      enabled: true,
    );
    _expectLocalizationVoiceAction(
      tester,
      key: const Key('revision3-localization-resolve-voice'),
      enabled: false,
    );
    await _navigateManagedHome(tester);
    await _tapManagedHomeTask(tester, const Key('managed-home-build'));
    _expectManagedSectionAction(
      tester,
      sectionId: 'build-release',
      actionId: 'build-voice-bundle',
      enabled: false,
    );
    expect(managed.voicePublishCalls, 0);
    expect(managed.voiceTargetPublishCalls, 0);
    expect(managed.voiceBuildCalls, 0);
  });

  testWidgets(
    'DataAsset installed-package browser reaches the exact managed lease',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_dataasset_browser_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      const projectId = '71717171717171717171717171717171';
      const revision = 4;
      final head = _head(revision);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\dataasset-browser'),
        projectId: projectId,
        projectRevision: revision,
        head: head,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onDataAssetList: (_) => const <AuthoringRevision3DataAssetStage>[],
        onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          expect(lease.head.canonicalJson, head.canonicalJson);
          return _homeDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
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
      await _navigateManagedDataAssets(tester);

      final browse = find.byKey(
        const Key('revision3-dataasset-browse-installed'),
      );
      expect(browse, findsOneWidget);
      expect(tester.widget<FilledButton>(browse).onPressed, isNotNull);
      expect(managed.dataAssetPackageIndexReadCalls, 0);
      await tester.tap(browse);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(
        find.byKey(const Key('installed-package-browser-result')),
        findsOneWidget,
      );
      expect(
        find.text('1 installed package candidate indexed'),
        findsOneWidget,
      );
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'reviewed DataAsset quick start returns its publication to the advanced registry',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_r3_reviewed_dataasset_game',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      final basis = revision3DataAssetNativeGoldenFixture();
      final fixture = Revision3DataAssetFixture.fromBasis(
        basisHead: basis.basisHead,
        basisProjectJson: basis.basisProjectJson,
        targetPath: _homeReviewedWolfTargetPath,
        selector: <String, Object?>{
          ..._homeReviewedWolfSelector(),
          'usmap_sha256': '3' * 64,
        },
        replacementHex:
            '00000000000026400000000000002840'
            '0000000000000000000000000000f03f',
      );
      final stage = AuthoringRevision3DataAssetStageListResult.fromJson(
        fixture.listResponse(),
        expectedHead: fixture.stagedHead,
      ).stages.single;
      var stages = <AuthoringRevision3DataAssetStage>[];
      ReviewedInstalledDataAssetEditIntent? publishedIntent;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\reviewed-dataasset-quick-start'),
        projectId: stage.projectId,
        projectRevision: 4,
        head: fixture.basisHead,
        canonicalProjectJsonValue: fixture.basisProjectJson,
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        onDataAssetList: (_) => List.unmodifiable(stages),
        onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          return _homeDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            targetPath: _homeReviewedWolfTargetPath,
          );
        },
        onInstalledDataAssetInspect:
            (lease, requestedGameRoot, expectedSnapshot, candidate) async {
              expect(requestedGameRoot, gameRoot.path);
              expect(
                identical(candidate, expectedSnapshot.index.candidates.single),
                isTrue,
              );
              return _homeInstalledDataAssetInspectionResult(
                expectedSnapshot: expectedSnapshot,
                candidate: candidate,
                inspection: _homeReviewedWolfInspectionResponse(),
              );
            },
        onReviewedInstalledDataAssetPublish:
            (lease, requestedGameRoot, intent) {
              expect(requestedGameRoot, gameRoot.path);
              expect(intent.expectedTargetPath, _homeReviewedWolfTargetPath);
              publishedIntent = intent;
              lease
                ..projectRevision = 5
                ..head = fixture.stagedHead
                ..canonicalProjectJsonValue = fixture.stagedProjectJson;
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
        gamePath: gameRoot.path,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedDataAssets(tester);

      await tester.tap(
        find.byKey(const Key('revision3-dataasset-browse-installed')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.byKey(const Key('installed-package-browser-reviewed-presets')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('installed-package-reviewed-open-0')),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(managed.installedDataAssetInspectCalls, 1);

      await tester.tap(find.byKey(const Key('dataasset-export-tile-0')));
      await tester.pump(const Duration(milliseconds: 300));
      final guidedEdit = find.byKey(const Key('reviewed-footstep-preset-edit'));
      await tester.ensureVisible(guidedEdit);
      await tester.tap(guidedEdit);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.enterText(
        find.byKey(const Key('reviewed-footstep-x')),
        '11',
      );
      await tester.enterText(
        find.byKey(const Key('reviewed-footstep-y')),
        '12',
      );
      await tester.tap(find.byKey(const Key('reviewed-footstep-preview')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
      await tester.pumpAndSettle();

      expect(managed.reviewedInstalledDataAssetPublishCalls, 1);
      expect(publishedIntent?.request.x, '11');
      expect(publishedIntent?.request.y, '12');
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, 5);
      expect(current.head.canonicalJson, fixture.stagedHead.canonicalJson);
      expect(managed.dataAssetListCalls, greaterThanOrEqualTo(2));
      final search = tester.widget<TextField>(
        find.byKey(const Key('revision3-dataasset-stage-search')),
      );
      expect(search.controller?.text, _homeReviewedWolfTargetPath);
      final stageTile = tester.widget<ExpansionTile>(
        find.byKey(
          const ValueKey(
            'revision3-dataasset-stage-'
            '$_homeReviewedWolfTargetPath',
          ),
        ),
      );
      expect(stageTile.controller?.isExpanded, isTrue);
      expect(find.text('DA_WolfFootsteps'), findsOneWidget);
    },
  );

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
      await _navigateManagedDataAssets(tester);
      expect(managed.dataAssetListCalls, 1);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-empty')),
        findsOneWidget,
      );

      final importProof = await _revealManagedDataAssetExpertAction(
        tester,
        const Key('revision3-dataasset-stage-add'),
      );
      await tester.tap(importProof);
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
      expect(
        find.text(
          'Choose the Gothic 1 Remake installation in Settings before building files.',
        ),
        findsOneWidget,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
              ),
            )
            .onPressed,
        isNull,
      );
      final removeButton = find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      );
      ScaffoldMessenger.of(
        tester.element(removeButton),
      ).removeCurrentSnackBar();
      await tester.pumpAndSettle();
      await Scrollable.ensureVisible(
        tester.element(removeButton),
        alignment: 0.5,
        duration: Duration.zero,
      );
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
      await _navigateManagedDataAssets(tester);

      final create = await _revealManagedDataAssetExpertAction(
        tester,
        const Key('revision3-dataasset-semantic-create'),
      );
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
      find.textContaining('Mod Studio project could not be opened:'),
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
      expect(
        find.byKey(const Key('managed-project-try-recovery')),
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
      expect(find.textContaining('safely reopen this project'), findsOneWidget);
      expect(
        find.byKey(const Key('managed-project-try-recovery')),
        findsOneWidget,
      );
      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsNothing,
      );
      for (final key in _managedPrimaryNavigationKeys) {
        expect(find.byKey(key), findsNothing);
      }
      expect(
        find.byKey(const Key('revision3-content-workspace-navigation')),
        findsNothing,
      );
      for (final key in const <Key>[
        Key('managed-home-story'),
        Key('managed-home-dialog-voice'),
        Key('managed-home-problems'),
        Key('managed-home-content'),
        Key('managed-home-build'),
      ]) {
        expect(find.byKey(key), findsNothing);
      }

      final menuFinder = find.byKey(const Key('project-menu'));
      final menu = tester.widget<PopupMenuButton<String>>(menuFinder);
      final verifyItem = menu
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .singleWhere((item) => item.key == const Key('project-save'));
      expect(verifyItem.enabled, isFalse);
      final exportItem = menu
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>()
          .singleWhere(
            (item) => item.key == const Key('project-export-managed-revision3'),
          );
      expect(exportItem.enabled, isFalse);

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
    'recovery blocks duplicate project actions and refreshes an advanced head',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const projectId = 'c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1';
      final previous = _recoverySnapshot(projectId: projectId, revision: 24);
      final recovered = _recoverySnapshot(projectId: projectId, revision: 25);
      final recoveryGate = Completer<void>();
      late final _FakeRecoverableManagedLease managed;
      managed = _FakeRecoverableManagedLease(
        root: Directory(r'C:\mods\recover-advanced'),
        projectId: projectId,
        projectRevision: 24,
        head: previous.head,
        canonicalProjectJsonValue: previous.projectJson,
        onRecovery: (lease) async {
          await recoveryGate.future;
          lease
            ..projectRevision = 25
            ..head = recovered.head
            ..canonicalProjectJsonValue = recovered.projectJson
            ..requiresReopenValue = false;
          return _recoveryCheckpoint(
            projectId: projectId,
            previousRevision: 24,
            previousHead: previous.head,
            recoveredRevision: 25,
            recoveredHead: recovered.head,
            recoveredProjectJson: recovered.projectJson,
          );
        },
      )..requiresReopenValue = true;
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
      final recoveryButton = find.byKey(
        const Key('managed-project-try-recovery'),
      );
      expect(recoveryButton, findsOneWidget);
      await tester.tap(recoveryButton);
      await tester.tap(recoveryButton);
      await tester.pump();

      expect(managed.recoveryCalls, 1);
      expect(
        find.byKey(const Key('managed-project-recovery-progress')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<IconButton>(find.byKey(const Key('managed-open-settings')))
            .onPressed,
        isNull,
      );
      final menuFinder = find.byKey(const Key('project-menu'));
      final projectItems = tester
          .widget<PopupMenuButton<String>>(menuFinder)
          .itemBuilder(tester.element(menuFinder))
          .whereType<PopupMenuItem<String>>();
      expect(projectItems.map((item) => item.enabled), everyElement(isFalse));

      recoveryGate.complete();
      await tester.pumpAndSettle();

      expect(managed.recoveryCalls, 1);
      expect(managed.verifyCalls, 0);
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('managed-project-try-recovery')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsOneWidget,
      );
      await _expandManagedTechnicalDetails(tester);
      expect(
        tester
            .widget<SelectableText>(
              find.byKey(const Key('managed-project-revision')),
            )
            .data,
        '25',
      );
      expect(
        find.text('Project recovery completed. You can continue working.'),
        findsOneWidget,
      );
    },
  );

  testWidgets('recovery also refreshes an unchanged durable head', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    const projectId = 'c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2';
    final snapshot = _recoverySnapshot(projectId: projectId, revision: 24);
    late final _FakeRecoverableManagedLease managed;
    managed = _FakeRecoverableManagedLease(
      root: Directory(r'C:\mods\recover-unchanged'),
      projectId: projectId,
      projectRevision: 24,
      head: snapshot.head,
      canonicalProjectJsonValue: snapshot.projectJson,
      onRecovery: (lease) {
        lease.requiresReopenValue = false;
        return _recoveryCheckpoint(
          projectId: projectId,
          previousRevision: 24,
          previousHead: snapshot.head,
          recoveredRevision: 24,
          recoveredHead: snapshot.head,
          recoveredProjectJson: snapshot.projectJson,
        );
      },
    )..requiresReopenValue = true;
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
    await tester.tap(find.byKey(const Key('managed-project-try-recovery')));
    await tester.pumpAndSettle();

    final current = coordinator.state as ManagedRevision3CurrentProjectState;
    expect(current.requiresReopen, isFalse);
    expect(current.projectRevision, 24);
    expect(current.head.canonicalJson, snapshot.head.canonicalJson);
    expect(managed.recoveryCalls, 1);
    expect(
      find.byKey(const Key('revision3-project-workspace')),
      findsOneWidget,
    );
  });

  testWidgets(
    'same-head recovery advances Story authority and loads Journey once',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final fixture = Revision3QuestOutlineFixture();
      final snapshot = _recoverySnapshot(
        projectId: revision3QuestOutlineProjectId,
        revision: fixture.projectRevision,
      );
      late final _FakeRecoverableManagedLease managed;
      managed = _FakeRecoverableManagedLease(
        root: Directory(r'C:\mods\recover-quest-journey'),
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: snapshot.head,
        canonicalProjectJsonValue: snapshot.projectJson,
        contentIndexBuilder: (_) => fixture.contentIndex(),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) =>
                AuthoringRevision3QuestTransitionsSeed.forProject(
                  currentProjectJson: fixture.projectJson,
                  questId: questId,
                  expectedQuestRevision: questRevision,
                  expectedModuleId: moduleId,
                  expectedModuleRevision: moduleRevision,
                ),
        onRecovery: (lease) {
          lease.requiresReopenValue = false;
          return _recoveryCheckpoint(
            projectId: revision3QuestOutlineProjectId,
            previousRevision: fixture.projectRevision,
            previousHead: snapshot.head,
            recoveredRevision: fixture.projectRevision,
            recoveredHead: snapshot.head,
            recoveredProjectJson: snapshot.projectJson,
          );
        },
      )..requiresReopenValue = true;
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
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsOneWidget,
      );
      expect(managed.questTransitionsSeedCalls, 0);

      await tester.tap(find.byKey(const Key('managed-project-try-recovery')));
      await tester.pumpAndSettle();
      await _navigateManagedStory(tester);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-quest-journey-panel')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<Revision3QuestJourneyView>(
              find.byType(Revision3QuestJourneyView),
            )
            .authorityEpoch,
        1,
      );
      expect(managed.questTransitionsSeedCalls, 1);
      await tester.pumpAndSettle();
      expect(managed.questTransitionsSeedCalls, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'retryable recovery failure stays locked and enables one later retry',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const projectId = 'c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3';
      final snapshot = _recoverySnapshot(projectId: projectId, revision: 24);
      late final _FakeRecoverableManagedLease managed;
      managed = _FakeRecoverableManagedLease(
        root: Directory(r'C:\mods\recover-failure'),
        projectId: projectId,
        projectRevision: 24,
        head: snapshot.head,
        canonicalProjectJsonValue: snapshot.projectJson,
        onRecovery: (_) => throw StateError('private repair details'),
      )..requiresReopenValue = true;
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
      final buttonFinder = find.byKey(
        const Key('managed-project-try-recovery'),
      );
      await tester.tap(buttonFinder);
      await tester.pumpAndSettle();

      expect(managed.recoveryCalls, 1);
      expect(
        find.byKey(const Key('managed-project-requires-reopen-warning')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('managed-project-recovery-error')),
        findsOneWidget,
      );
      final warningText = tester
          .widgetList<Text>(
            find.descendant(
              of: find.byKey(
                const Key('managed-project-requires-reopen-warning'),
              ),
              matching: find.byType(Text),
            ),
          )
          .map((text) => text.data ?? text.textSpan?.toPlainText() ?? '')
          .join(' ')
          .toLowerCase();
      expect(warningText, isNot(contains('private repair details')));
      expect(warningText, isNot(contains('journal')));
      expect(warningText, isNot(contains('hash')));
      expect(warningText, isNot(contains('repairoutcome')));
      expect(tester.widget<FilledButton>(buttonFinder).onPressed, isNotNull);

      await tester.tap(buttonFinder);
      await tester.pumpAndSettle();
      expect(managed.recoveryCalls, 2);
      expect(managed.verifyCalls, 0);
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'unsupported recovery is terminal and keeps the close-open fallback',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const projectId = 'c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4';
      final snapshot = _recoverySnapshot(projectId: projectId, revision: 24);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\recover-unsupported'),
        projectId: projectId,
        projectRevision: 24,
        head: snapshot.head,
        canonicalProjectJsonValue: snapshot.projectJson,
      )..requiresReopenValue = true;
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
      final buttonFinder = find.byKey(
        const Key('managed-project-try-recovery'),
      );
      await tester.tap(buttonFinder);
      await tester.pumpAndSettle();

      expect(
        find.textContaining('Recovery is not available for this project'),
        findsOneWidget,
      );
      expect(
        find.textContaining('close and open the project again'),
        findsWidgets,
      );
      expect(tester.widget<FilledButton>(buttonFinder).onPressed, isNull);
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'a stale rendered recovery cannot call or affect a switched project',
    (tester) async {
      await _setDesktopTestSurface(tester);
      const oldProjectId = 'c5c5c5c5c5c5c5c5c5c5c5c5c5c5c5c5';
      const newProjectId = 'c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6';
      final oldSnapshot = _recoverySnapshot(
        projectId: oldProjectId,
        revision: 24,
      );
      final newSnapshot = _recoverySnapshot(
        projectId: newProjectId,
        revision: 7,
      );
      final oldRoot = Directory(r'C:\mods\recover-old');
      final newRoot = Directory(r'C:\mods\recover-new');
      late final _FakeRecoverableManagedLease oldProject;
      oldProject = _FakeRecoverableManagedLease(
        root: oldRoot,
        projectId: oldProjectId,
        projectRevision: 24,
        head: oldSnapshot.head,
        canonicalProjectJsonValue: oldSnapshot.projectJson,
        onRecovery: (_) => throw StateError('must not be called'),
      )..requiresReopenValue = true;
      final newProject = _FakeManagedLease(
        root: newRoot,
        projectId: newProjectId,
        projectRevision: 7,
        head: newSnapshot.head,
        canonicalProjectJsonValue: newSnapshot.projectJson,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async =>
            root.path == oldRoot.path ? oldProject : newProject,
      );
      await coordinator.openManagedRevision3(oldRoot);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      final staleOnPressed = tester
          .widget<FilledButton>(
            find.byKey(const Key('managed-project-try-recovery')),
          )
          .onPressed!;
      await coordinator.closeCurrent();
      await coordinator.openManagedRevision3(newRoot);
      staleOnPressed();
      await tester.pumpAndSettle();

      expect(oldProject.recoveryCalls, 0);
      expect(oldProject.closeCalls, 1);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectId, newProjectId);
      expect(current.projectRevision, 7);
      expect(current.requiresReopen, isFalse);
      expect(
        find.byKey(const Key('managed-project-recovery-error')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'project command bar remembers the friendly index name and follows the current section',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\project-command-orientation'),
        projectId: 'd1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1',
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
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

      expect(managed.contentReadCalls, greaterThanOrEqualTo(1));
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.projectNameKey))
            .data,
        'Home Quest project',
      );
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.sectionKey))
            .data,
        'Current section: Home',
      );

      await _navigateManagedStory(tester);

      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.sectionKey))
            .data,
        'Current section: Story',
      );
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.projectNameKey))
            .data,
        'Home Quest project',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'project command Search opens focused Search all without eager source reads',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final gameRoot = Directory.systemTemp.createTempSync(
        'gore_project_command_search_game_',
      );
      Directory(p.join(gameRoot.path, 'G1R')).createSync();
      addTearDown(() {
        if (gameRoot.existsSync()) gameRoot.deleteSync(recursive: true);
      });
      var baseCatalogCalls = 0;
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\project-command-search'),
        projectId: 'd2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2',
        projectRevision: 8,
        head: _head(8),
        contentIndexBuilder: (lease) => _globalSearchContentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          targetEntityId: 'd3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3',
        ),
        onDataAssetPackageIndexRead: (lease, requestedGameRoot) async {
          expect(requestedGameRoot, gameRoot.path);
          return _homeDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
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
        loadBaseGameCatalog: (_) async {
          baseCatalogCalls++;
          return _baseGameCatalog();
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      final contentReadsBeforeCommand = managed.contentReadCalls;
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
      await tester.pumpAndSettle();

      expect(
        find.byKey(
          const Key('revision3-scoped-content-browser-page-all-sources'),
        ),
        findsOneWidget,
      );
      final query = find.byKey(
        const Key('revision3-global-content-search-field'),
      );
      expect(query, findsOneWidget);
      expect(tester.widget<TextField>(query).focusNode?.hasFocus, isTrue);
      expect(managed.contentReadCalls, contentReadsBeforeCommand);
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await tester.tap(
        find.byKey(const Key('revision3-global-content-search-submit')),
      );
      await tester.pumpAndSettle();
      expect(managed.contentReadCalls, contentReadsBeforeCommand);
      expect(baseCatalogCalls, 0);
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      await tester.enterText(query, 'asghan');
      await tester.tap(
        find.byKey(const Key('revision3-global-content-search-submit')),
      );
      await tester.pumpAndSettle();

      expect(managed.contentReadCalls, greaterThan(contentReadsBeforeCommand));
      expect(baseCatalogCalls, 1);
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'project command Create explains game-gated choices and keeps Dialog line available',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\project-command-create'),
        projectId: 'd4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4',
        projectRevision: 9,
        head: _head(9),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
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
      await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
      await tester.pumpAndSettle();

      final npc = find.byKey(const Key('managed-project-create-npc'));
      final quest = find.byKey(
        const Key('managed-project-create-quest-opening'),
      );
      final dialog = find.byKey(
        const Key('managed-project-create-dialog-line'),
      );
      expect(npc, findsOneWidget);
      expect(quest, findsOneWidget);
      expect(dialog, findsOneWidget);
      final reason = AppLocalizations.of(
        tester.element(npc),
      ).managedDashboardMissingGameDescription;
      for (final gated in <Finder>[npc, quest]) {
        final tile = tester.widget<ListTile>(gated);
        expect(tile.enabled, isFalse);
        expect(tile.onTap, isNull);
        expect((tile.subtitle! as Text).data, reason);
      }
      final dialogTile = tester.widget<ListTile>(dialog);
      expect(dialogTile.enabled, isTrue);
      expect(dialogTile.onTap, isNotNull);
      expect((dialogTile.subtitle! as Text).data, isNot(reason));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('project command Problems opens Validate and Test', (
    tester,
  ) async {
    await _setDesktopTestSurface(tester);
    final managed = _FakeManagedLease(
      root: Directory(r'C:\mods\project-command-problems'),
      projectId: 'd5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5',
      projectRevision: 10,
      head: _head(10),
      contentIndexBuilder: (lease) => _contentIndex(
        projectId: lease.projectId,
        revision: lease.projectRevision,
      ),
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
    await tester.tap(find.byKey(Revision3ProjectCommandBar.problemsKey));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-workspace-page-validate-test')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-problems-view')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<Text>(find.byKey(Revision3ProjectCommandBar.sectionKey))
          .data,
      'Current section: Validate & Test',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'stale project command fails closed after a project switch with an equal checkpoint',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final oldRoot = Directory(r'C:\mods\project-command-stale-old');
      final newRoot = Directory(r'C:\mods\project-command-stale-new');
      final oldProject = _FakeManagedLease(
        root: oldRoot,
        projectId: 'd6d6d6d6d6d6d6d6d6d6d6d6d6d6d6d6',
        projectRevision: 11,
        head: _head(11),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
          projectName: 'Old friendly project',
        ),
      );
      final newProject = _FakeManagedLease(
        root: newRoot,
        projectId: 'd7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7',
        projectRevision: 11,
        head: _head(11),
        contentIndexBuilder: (_) => throw StateError('new index unavailable'),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (root) async =>
            root.path == oldRoot.path ? oldProject : newProject,
      );
      await coordinator.openManagedRevision3(oldRoot);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      final oldContentReads = oldProject.contentReadCalls;
      final staleOnPressed = tester
          .widget<OutlinedButton>(
            find.byKey(Revision3ProjectCommandBar.searchKey),
          )
          .onPressed!;

      await coordinator.closeCurrent();
      await coordinator.openManagedRevision3(newRoot);
      staleOnPressed();
      await tester.pumpAndSettle();

      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectId, newProject.projectId);
      expect(current.projectRevision, oldProject.projectRevision);
      expect(oldProject.closeCalls, 1);
      expect(oldProject.contentReadCalls, oldContentReads);
      expect(
        find.byKey(
          const Key('revision3-scoped-content-browser-page-all-sources'),
        ),
        findsNothing,
      );
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.projectNameKey))
            .data,
        'project-command-stale-new',
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'project command bar survives compact German two-hundred-percent text',
    (tester) async {
      await _setTestSurface(tester, const Size(360, 480));
      tester.platformDispatcher.textScaleFactorTestValue = 2;
      addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\project-command-compact'),
        projectId: 'd8d8d8d8d8d8d8d8d8d8d8d8d8d8d8d8',
        projectRevision: 12,
        head: _head(12),
        contentIndexBuilder: (lease) => _contentIndex(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async => null,
      );
      container.read(localeProvider.notifier).setLocale('de');
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();

      expect(find.byKey(Revision3ProjectCommandBar.rootKey), findsOneWidget);
      expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
      expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsOneWidget);
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.sectionKey))
            .data,
        'Aktueller Bereich: Start',
      );

      await tester.tap(find.byKey(Revision3ProjectCommandBar.moreKey));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Revision3ProjectCommandBar.compactSettingsKey),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('managed-settings-dialog')), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('managed-settings-close')));
      await tester.pumpAndSettle();

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-tab-settings-expert'),
      );
      expect(
        tester
            .widget<Text>(find.byKey(Revision3ProjectCommandBar.sectionKey))
            .data,
        'Aktueller Bereich: Einstellungen & Expertenmodus',
      );
      await _expandManagedTechnicalDetails(tester);
      expect(
        find.byKey(const Key('managed-project-technical-details-scroll')),
        findsOneWidget,
      );
      expect(find.byKey(const Key('managed-project-head')), findsOneWidget);
      expect(tester.takeException(), isNull);
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
      expect(find.text('Mod Studio project opened.'), findsNothing);
    },
  );
}

ProviderContainer _container({
  required CurrentProjectCoordinator coordinator,
  required ManagedRevision3DirectoryPicker pickManaged,
  Revision3ProjectImportSourcePicker? pickProjectBackup,
  Revision3ProjectImportNativeInspector? inspectProjectBackup,
  Revision3ProjectImportNativeDestinationImporter? restoreProjectBackup,
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
  Revision3BaseGameContentCatalogLoader? loadBaseGameCatalog,
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
    if (pickProjectBackup != null)
      managedRevision3ProjectBackupPickerProvider.overrideWithValue(
        pickProjectBackup,
      ),
    if (inspectProjectBackup != null)
      managedRevision3ProjectBackupInspectorProvider.overrideWithValue(
        inspectProjectBackup,
      ),
    if (restoreProjectBackup != null)
      managedRevision3ProjectBackupRestorerProvider.overrideWithValue(
        restoreProjectBackup,
      ),
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
    if (loadBaseGameCatalog != null)
      revision3BaseGameContentCatalogLoaderProvider.overrideWithValue(
        loadBaseGameCatalog,
      ),
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

Future<void> _setDesktopTestSurface(WidgetTester tester) =>
    _setTestSurface(tester, const Size(1600, 900));

Future<void> _setNarrowShortTestSurface(WidgetTester tester) =>
    _setTestSurface(tester, const Size(640, 420));

Future<void> _setTestSurface(WidgetTester tester, Size size) async {
  tester.view.physicalSize = size;
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

Future<void> _expandManagedTechnicalDetails(WidgetTester tester) async {
  if (find.byKey(const Key('managed-project-revision')).evaluate().isNotEmpty) {
    return;
  }
  var details = find.byKey(const Key('managed-project-technical-details'));
  if (details.evaluate().isEmpty) {
    await _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-tab-settings-expert'),
    );
    details = find.byKey(const Key('managed-project-technical-details'));
  }
  expect(details, findsOneWidget);
  await tester.ensureVisible(details);
  await tester.pumpAndSettle();
  if (details.hitTestable().evaluate().isNotEmpty) {
    await tester.tap(details);
  } else {
    final scroll = find.byKey(
      const Key('managed-project-technical-details-scroll'),
    );
    expect(scroll, findsOneWidget);
    final visible = tester.getRect(details).intersect(tester.getRect(scroll));
    expect(visible.isEmpty, isFalse);
    await tester.tapAt(visible.center);
  }
  await tester.pumpAndSettle();
}

const _managedPrimaryNavigationKeys = <Key>[
  Key('revision3-project-workspace-tab-home'),
  Key('revision3-project-workspace-tab-content'),
  Key('revision3-project-workspace-tab-story'),
  Key('revision3-project-workspace-tab-world'),
  Key('revision3-project-workspace-tab-localization-voice'),
  Key('revision3-project-workspace-tab-validate-test'),
  Key('revision3-project-workspace-tab-build-release'),
  Key('revision3-project-workspace-tab-history'),
  Key('revision3-project-workspace-tab-settings-expert'),
];

void _expectExactCreatedNpcStoryDialogVoice(WidgetTester tester) {
  expect(
    find.byKey(const Key('revision3-project-workspace-page-story')),
    findsOneWidget,
  );
  final selectedNpc = find.byKey(
    const Key('revision3-story-workspace-entity-$_homeCreatedNpcId'),
  );
  expect(selectedNpc, findsOneWidget);
  expect(tester.widget<ListTile>(selectedNpc).selected, isTrue);
  final dialogVoice = find.byKey(
    const Key('revision3-story-workbench-tab-dialogVoice-$_homeCreatedNpcId'),
  );
  expect(dialogVoice, findsOneWidget);
  expect(tester.widget<ChoiceChip>(dialogVoice).selected, isTrue);
  expect(
    find.byKey(const Key('revision3-npc-dialog-voice-panel')),
    findsOneWidget,
  );
  expect(
    find.byKey(const Key('revision3-npc-greeting-new-line')),
    findsOneWidget,
  );
  expect(find.text('North Gate Guard'), findsWidgets);
  expect(
    find.textContaining('could not be selected at its exact project revision'),
    findsNothing,
  );
  expect(find.textContaining('Character draft saved'), findsOneWidget);
  expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
  expect(find.text('Build / Deploy'), findsNothing);
  for (final technicalIdentity in const <String>[
    'g1r:npc:om_grd_asghan_263',
    _homeCreatedNpcId,
    _homeCreatedNpcModuleId,
    _homeCreatedNpcUniqueName,
    _homeCreatedNpcModuleNamespace,
  ]) {
    expect(find.text(technicalIdentity), findsNothing);
  }
  expect(tester.takeException(), isNull);
}

Future<void> _navigateManagedHome(WidgetTester tester) =>
    _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-tab-home'),
    );

Future<void> _navigateManagedStory(WidgetTester tester) =>
    _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-tab-story'),
    );

Future<void> _navigateManagedContent(WidgetTester tester) async {
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-project-workspace-tab-content'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-content-workspace-nav-library'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-scoped-content-browser-nav-this-mod'),
  );
  expect(
    find.byKey(const Key('revision3-scoped-content-browser-page-this-mod')),
    findsOneWidget,
  );
}

Future<void> _navigateManagedBaseGameContent(WidgetTester tester) async {
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-project-workspace-tab-content'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-content-workspace-nav-library'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-scoped-content-browser-nav-base-game'),
  );
  expect(
    find.byKey(const Key('revision3-scoped-content-browser-page-base-game')),
    findsOneWidget,
  );
}

Future<void> _navigateManagedInstalledContent(WidgetTester tester) async {
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-project-workspace-tab-content'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-content-workspace-nav-library'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-scoped-content-browser-nav-installed'),
  );
  expect(
    find.byKey(const Key('revision3-scoped-content-browser-page-installed')),
    findsOneWidget,
  );
}

Future<void> _navigateManagedDataAssets(WidgetTester tester) async {
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-project-workspace-tab-content'),
  );
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-content-workspace-nav-data-assets'),
  );
  expect(
    find.byKey(const Key('revision3-content-workspace-page-data-assets')),
    findsOneWidget,
  );
}

Future<Finder> _revealManagedDataAssetExpertAction(
  WidgetTester tester,
  Key actionKey,
) async {
  final expertTools = find.byKey(const Key('revision3-dataasset-expert-tools'));
  expect(expertTools, findsOneWidget);
  await tester.ensureVisible(expertTools);
  await tester.pump();
  await tester.tap(expertTools);
  await tester.pumpAndSettle();

  final action = find.byKey(actionKey);
  expect(action, findsOneWidget);
  await tester.ensureVisible(action);
  await tester.pump();
  return action;
}

Future<void> _navigateManagedWorkspace(WidgetTester tester, Key key) async {
  final destination = find.byKey(key);
  expect(destination, findsOneWidget);
  await tester.ensureVisible(destination);
  await tester.pumpAndSettle();
  if (destination.hitTestable().evaluate().isNotEmpty) {
    await tester.tap(destination);
  } else {
    final tabBar = find.byKey(const Key('revision3-project-workspace-tabbar'));
    expect(tabBar, findsOneWidget);
    final viewport = find
        .descendant(of: tabBar, matching: find.byType(Scrollable))
        .first;
    final visible = tester
        .getRect(destination)
        .intersect(tester.getRect(viewport));
    expect(visible.isEmpty, isFalse);
    await tester.tapAt(visible.center);
  }
  await tester.pumpAndSettle();
}

void _selectManagedWorkspaceTabProgrammatically(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section,
) {
  final tabBar = tester.widget<TabBar>(
    find.byKey(
      const Key('revision3-project-workspace-tabbar'),
      skipOffstage: false,
    ),
  );
  expect(tabBar.onTap, isNotNull);
  tabBar.onTap!(section.index);
}

Future<void> _navigateManagedLocalizationVoice(
  WidgetTester tester, {
  bool settle = true,
}) async {
  const destinationKey = Key(
    'revision3-project-workspace-tab-localization-voice',
  );
  final destination = find.byKey(destinationKey);
  expect(destination, findsOneWidget);
  await tester.ensureVisible(destination);
  await tester.tap(destination);
  if (settle) {
    await tester.pumpAndSettle();
    await _switchManagedLocalizationVoiceToProjectTexts(tester);
  } else {
    await tester.pump();
  }
}

Future<void> _switchManagedLocalizationVoiceToProjectTexts(
  WidgetTester tester,
) async {
  final mode = find.byKey(const Key('revision3-localization-voice-mode'));
  if (mode.evaluate().isEmpty) return;
  final workspace = tester.widget<Revision3LocalizationVoiceWorkspace>(
    find.byType(Revision3LocalizationVoiceWorkspace),
  );
  final projectTexts = find.descendant(
    of: mode,
    matching: find.text(workspace.copy.projectTextsLabel),
  );
  expect(projectTexts, findsOneWidget);
  await tester.ensureVisible(projectTexts);
  await tester.tap(projectTexts);
  await tester.pumpAndSettle();
}

Future<void> _scrollManagedVoiceQueueUntilVisible(
  WidgetTester tester,
  Finder target,
) async {
  final scrollable = find
      .descendant(
        of: find.byKey(const Key('revision3-voice-production-queue')),
        matching: find.byType(Scrollable),
      )
      .first;
  await tester.scrollUntilVisible(target, 300, scrollable: scrollable);
  await tester.pump();
  expect(target.hitTestable(), findsOneWidget);
}

Future<void> _scrollManagedEditorUntilVisible(
  WidgetTester tester,
  Finder target,
) async {
  final scrollable = find
      .descendant(
        of: find.byKey(const Key('revision3-localization-editor-scroll')),
        matching: find.byType(Scrollable),
      )
      .first;
  final viewportHeight =
      tester.view.physicalSize.height / tester.view.devicePixelRatio;
  for (var attempt = 0; attempt < 20; attempt++) {
    final center = tester.getCenter(target, warnIfMissed: false);
    if (center.dy >= 0 && center.dy <= viewportHeight) return;
    await tester.drag(scrollable, const Offset(0, -220));
    await tester.pump();
  }
  fail('managed editor target did not become visible');
}

Future<void> _openStoryWorkbenchEntity(
  WidgetTester tester,
  String entityId,
) async {
  final entity = find.byKey(Key('revision3-content-entity-$entityId'));
  expect(entity, findsOneWidget);
  await tester.ensureVisible(entity);
  await tester.pumpAndSettle();
  expect(entity.hitTestable(), findsOneWidget);
  await tester.tap(entity);
  await tester.pumpAndSettle();
  final openStory = find.byKey(Key('revision3-content-open-story-$entityId'));
  expect(openStory, findsOneWidget);
  await tester.ensureVisible(openStory);
  await tester.pumpAndSettle();
  expect(openStory.hitTestable(), findsOneWidget);
  await tester.tap(openStory);
  await tester.pumpAndSettle();
  expect(find.byKey(const Key('revision3-story-workspace')), findsOneWidget);
  expect(
    find.byKey(ValueKey('revision3-content-entity-details-$entityId')),
    findsOneWidget,
  );
}

Finder _storyWorkbenchAction(Key key) => find.byKey(key, skipOffstage: false);

Future<void> _revealWorkbenchAction(WidgetTester tester, Finder action) async {
  expect(action, findsOneWidget);
  final scrollable = find
      .ancestor(of: action, matching: find.byType(Scrollable))
      .last;
  expect(scrollable, findsOneWidget);
  await tester.scrollUntilVisible(action, 120, scrollable: scrollable);
  await tester.pumpAndSettle();
  expect(
    tester.getRect(action).intersect(tester.getRect(scrollable)).isEmpty,
    isFalse,
  );
}

Finder _workbenchActionTile(Finder action) => find.descendant(
  of: action,
  matching: find.byType(ListTile),
  skipOffstage: false,
);

ListTile _workbenchActionTileWidget(WidgetTester tester, Finder action) {
  final tile = _workbenchActionTile(action);
  expect(tile, findsOneWidget);
  return tester.widget<ListTile>(tile);
}

Future<void> _tapWorkbenchAction(WidgetTester tester, Finder action) async {
  final tile = _workbenchActionTile(action);
  final scrollable = find
      .ancestor(of: action, matching: find.byType(Scrollable))
      .last;
  final visible = tester.getRect(tile).intersect(tester.getRect(scrollable));
  expect(visible.isEmpty, isFalse);
  await tester.tapAt(visible.center);
}

Finder _managedSectionAction({
  required String sectionId,
  required String actionId,
}) => find.byKey(Key('revision3-project-section-$sectionId-action-$actionId'));

void _expectManagedSectionAction(
  WidgetTester tester, {
  required String sectionId,
  required String actionId,
  required bool enabled,
}) {
  final action = _managedSectionAction(
    sectionId: sectionId,
    actionId: actionId,
  );
  expect(action, findsOneWidget);
  final inkWell = find.descendant(of: action, matching: find.byType(InkWell));
  expect(inkWell, findsOneWidget);
  expect(tester.widget<InkWell>(inkWell).onTap, enabled ? isNotNull : isNull);
}

void _expectLocalizationVoiceAction(
  WidgetTester tester, {
  required Key key,
  required bool enabled,
}) {
  final action = find.byKey(key);
  expect(action, findsOneWidget);
  final actionWidget = tester.widget(action);
  if (actionWidget is FilledButton) {
    expect(actionWidget.onPressed, enabled ? isNotNull : isNull);
    return;
  }
  final button = find.descendant(
    of: action,
    matching: find.byType(OutlinedButton),
  );
  expect(button, findsOneWidget);
  expect(
    tester.widget<OutlinedButton>(button).onPressed,
    enabled ? isNotNull : isNull,
  );
}

Future<void> _tapManagedHomeTask(WidgetTester tester, Key key) async {
  final task = find.byKey(key);
  expect(task, findsOneWidget);
  expect(tester.widget<ListTile>(task).onTap, isNotNull);
  await tester.ensureVisible(task);
  await tester.pumpAndSettle();
  await tester.tap(task);
  await tester.pumpAndSettle();
}

Future<void> _tapLocalizationVoiceAction(WidgetTester tester, Key key) async {
  final action = find.byKey(key);
  expect(action, findsOneWidget);
  final actionWidget = tester.widget(action);
  if (actionWidget is FilledButton) {
    expect(actionWidget.onPressed, isNotNull);
    await tester.ensureVisible(action);
    await tester.tap(action);
    return;
  }
  final button = find.descendant(
    of: action,
    matching: find.byType(OutlinedButton),
  );
  expect(button, findsOneWidget);
  expect(tester.widget<OutlinedButton>(button).onPressed, isNotNull);
  await tester.ensureVisible(button);
  await tester.tap(button);
}

Future<void> _openAdvancedQuestCreation(WidgetTester tester) async {
  final menu = find.byKey(
    const Key('revision3-story-workspace-create-advanced'),
  );
  expect(menu, findsOneWidget);
  await tester.ensureVisible(menu);
  await tester.tap(menu);
  await tester.pumpAndSettle();
  final createQuest = find.byKey(
    const Key('revision3-story-workspace-create-quest'),
  );
  expect(createQuest, findsOneWidget);
  expect(tester.widget<PopupMenuItem<dynamic>>(createQuest).enabled, isTrue);
  await tester.tap(createQuest);
}

Future<void> _pumpUntilFound(
  WidgetTester tester,
  Finder target, {
  int maxPumps = 40,
}) async {
  for (var pump = 0; pump < maxPumps; pump++) {
    await tester.pump(const Duration(milliseconds: 50));
    if (target.evaluate().isNotEmpty) return;
  }
  expect(target, findsOneWidget);
}

Future<void> _dragUntilFound(
  WidgetTester tester, {
  required Finder scrollable,
  required Finder target,
  int maxDrags = 8,
}) async {
  for (var drag = 0; drag < maxDrags; drag++) {
    if (target.evaluate().isNotEmpty) return;
    await tester.drag(scrollable, const Offset(0, -180));
    await tester.pump();
  }
  expect(target, findsOneWidget);
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

typedef _DialogLocalizationReadCallback =
    FutureOr<AuthoringRevision3DialogLocalizationReadResult> Function(
      _FakeManagedLease lease,
      String localizationId,
      int expectedLocalizationRevision,
      String expectedLocId,
    );
typedef _DialogLocalizationEditSeedCallback =
    FutureOr<AuthoringRevision3DialogLocalizationEditSeed> Function(
      _FakeManagedLease lease,
      String localizationId,
      int expectedLocalizationRevision,
      String expectedLocId,
    );
typedef _DialogLocalizationEditPublishCallback =
    FutureOr<Revision3DialogLocalizationEditPublication> Function(
      _FakeManagedLease lease,
      Revision3DialogLocalizationEditTechnicalPlan plan,
    );
typedef _InstalledDataAssetInspectCallback =
    FutureOr<AuthoringRevision3InstalledDataAssetInspectionResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
      AuthoringRevision3DataAssetPackageCandidate candidate,
    );
typedef _ReviewedInstalledDataAssetPublishCallback =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      ReviewedInstalledDataAssetEditIntent intent,
    );
typedef _RecoveryCallback =
    FutureOr<ManagedRevision3RecoveryCheckpoint> Function(
      _FakeRecoverableManagedLease lease,
    );
typedef _ProjectExportCallback =
    FutureOr<AuthoringRevision3ExactSnapshotExportResultV2> Function(
      _FakeExportManagedLease lease,
      String output,
    );
typedef _QuestTranscriptCreateCallback =
    FutureOr<Revision3QuestTranscriptPublication> Function(
      _FakeQuestTranscriptManagedLease lease,
      Revision3QuestTranscriptCreateTechnicalPlan plan,
    );
typedef _NpcGreetingCreateCallback =
    FutureOr<Revision3NpcGreetingPublication> Function(
      _FakeNpcGreetingManagedLease lease,
      Revision3NpcGreetingCreateTechnicalPlan plan,
    );
typedef _DialogVoiceSlotCreationCallback =
    FutureOr<Revision3DialogVoiceSlotCreationPublication> Function(
      _FakeDialogVoiceSlotCreationManagedLease lease,
      Revision3DialogVoiceSlotCreationTechnicalPlan plan,
    );

class _FakeManagedLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3DialogLocalizationReadLease,
        ManagedRevision3DialogLocalizationEditLease,
        ManagedRevision3VoiceTakeStatusLease {
  _FakeManagedLease({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    this.canonicalProjectJsonValue = '{}',
    this.verificationError,
    this.onNpcPublish,
    this.onQuestPublish,
    this.onQuestSourceInspection,
    this.onNpcSourceInspection,
    this.onManagedCompilerCheck,
    this.onQuestOutlinePublish,
    this.onQuestTransitionsSeed,
    this.onQuestTransitionsPublish,
    this.onQuestContextSeed,
    this.onQuestContextPublish,
    this.onDialogLocalizationRead,
    this.onDialogLocalizationEditSeed,
    this.onDialogLocalizationEditPublish,
    this.onDialogLinePublish,
    this.onVoicePublish,
    this.onVoiceSelectionPublish,
    this.onVoiceStatusPublish,
    this.onVoiceTargetPublish,
    this.onVoicePlan,
    this.onVoiceBuild,
    this.onDataAssetList,
    this.onDataAssetPublish,
    this.onDataAssetSemanticPublish,
    this.onDataAssetRemove,
    this.onDataAssetPackageIndexRead,
    this.onInstalledDataAssetInspect,
    this.onReviewedInstalledDataAssetPublish,
    this.onContentIndexRead,
    this.contentIndexBuilder,
    this.closeError,
  });

  @override
  final Directory root;
  @override
  final String projectId;
  @override
  String get canonicalProjectJson => canonicalProjectJsonValue;
  String canonicalProjectJsonValue;
  @override
  int projectRevision;
  @override
  AuthoringWorkingHead head;
  final Object? verificationError;
  final Object? closeError;
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
  final Future<AuthoringRevision3QuestSourceInspectionResult> Function(
    _FakeManagedLease lease,
    String gameRoot,
    String questId,
  )?
  onQuestSourceInspection;
  final Future<AuthoringRevision3NpcSourceInspectionResult> Function(
    _FakeManagedLease lease,
    String npcId,
  )?
  onNpcSourceInspection;
  final Future<ManagedRevision3CompilerCheckReceipt> Function(
    _FakeManagedLease lease,
    AuthoringRevision3ManagedCompilerEntityKind entityKind,
    String gameRoot,
    String entityId,
    int expectedEntityRevision,
    String expectedModuleId,
    int expectedModuleRevision,
  )?
  onManagedCompilerCheck;
  final Revision3QuestOutlineEditPublication Function(
    _FakeManagedLease lease,
    Revision3QuestOutlineEditInput input,
  )?
  onQuestOutlinePublish;
  final AuthoringRevision3QuestTransitionsSeed Function(
    _FakeManagedLease lease,
    String questId,
    int questRevision,
    String moduleId,
    int moduleRevision,
  )?
  onQuestTransitionsSeed;
  final Revision3QuestTransitionsEditPublication Function(
    _FakeManagedLease lease,
    Revision3QuestTransitionsEditTechnicalPlan plan,
  )?
  onQuestTransitionsPublish;
  final AuthoringRevision3QuestContextSeed Function(
    _FakeManagedLease lease,
    String questId,
    int questRevision,
    String moduleId,
    int moduleRevision,
    String parentRuntimeClass,
    String giverRuntimeUniqueName,
  )?
  onQuestContextSeed;
  final Revision3QuestContextEditPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3QuestContextEditTechnicalPlan plan,
  )?
  onQuestContextPublish;
  final _DialogLocalizationReadCallback? onDialogLocalizationRead;
  final _DialogLocalizationEditSeedCallback? onDialogLocalizationEditSeed;
  final _DialogLocalizationEditPublishCallback? onDialogLocalizationEditPublish;
  final Revision3DialogLineEntryPublication Function(
    _FakeManagedLease lease,
    Revision3DialogLineEntryTechnicalPlan plan,
  )?
  onDialogLinePublish;
  final Revision3VoiceTakePublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3VoiceTakeTechnicalPlan plan,
  )?
  onVoicePublish;
  final Revision3VoiceTakeSelectionPublication Function(
    _FakeManagedLease lease,
    Revision3VoiceTakeSelectionTechnicalPlan plan,
  )?
  onVoiceSelectionPublish;
  final Revision3VoiceTakeStatusPublication Function(
    _FakeManagedLease lease,
    Revision3VoiceTakeStatusTechnicalPlan plan,
  )?
  onVoiceStatusPublish;
  final Revision3VoiceTargetPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3VoiceTargetTechnicalPlan plan,
  )?
  onVoiceTargetPublish;
  final AuthoringRevision3VoiceBuildPlanResult Function(
    _FakeManagedLease lease,
  )?
  onVoicePlan;
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
  Future<AuthoringRevision3DataAssetPackageIndexResult> Function(
    _FakeManagedLease lease,
    String gameRoot,
  )?
  onDataAssetPackageIndexRead;
  final _InstalledDataAssetInspectCallback? onInstalledDataAssetInspect;
  final _ReviewedInstalledDataAssetPublishCallback?
  onReviewedInstalledDataAssetPublish;
  final Future<Revision3ContentIndex> Function(_FakeManagedLease lease)?
  onContentIndexRead;
  final Revision3ContentIndex Function(_FakeManagedLease lease)?
  contentIndexBuilder;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int npcPublishCalls = 0;
  int questPublishCalls = 0;
  int questSourceInspectionCalls = 0;
  int npcSourceInspectionCalls = 0;
  int managedCompilerCheckCalls = 0;
  final List<String> npcSourceInspectionNpcIds = <String>[];
  int questOutlinePublishCalls = 0;
  int questTransitionsSeedCalls = 0;
  int questTransitionsPublishCalls = 0;
  int questContextSeedCalls = 0;
  int questContextPublishCalls = 0;
  int dialogLocalizationReadCalls = 0;
  int dialogLocalizationEditSeedCalls = 0;
  int dialogLocalizationEditPublishCalls = 0;
  int dialogLinePublishCalls = 0;
  int voicePublishCalls = 0;
  int voiceSelectionPublishCalls = 0;
  int voiceStatusPublishCalls = 0;
  int voiceTargetPublishCalls = 0;
  int voicePlanCalls = 0;
  int voiceBuildCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetPublishCalls = 0;
  int dataAssetSemanticPublishCalls = 0;
  int dataAssetRemoveCalls = 0;
  int dataAssetPackageIndexReadCalls = 0;
  int installedDataAssetInspectCalls = 0;
  int reviewedInstalledDataAssetPublishCalls = 0;
  int closeCalls = 0;

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  Future<void> close() async {
    closeCalls++;
    final error = closeError;
    if (error != null) throw error;
  }

  @override
  Future<Revision3ContentIndex> readContentIndex() async {
    contentReadCalls++;
    final read = onContentIndexRead;
    if (read != null) return read(this);
    return contentIndexBuilder?.call(this) ??
        (throw StateError('fake managed lease has no content index'));
  }

  @override
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    dialogLocalizationReadCalls++;
    final read = onDialogLocalizationRead;
    if (read == null) {
      throw StateError('fake managed lease has no localization reader');
    }
    return read(
      this,
      localizationId,
      expectedLocalizationRevision,
      expectedLocId,
    );
  }

  @override
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    dialogLocalizationEditSeedCalls++;
    final read = onDialogLocalizationEditSeed;
    if (read == null) {
      throw StateError('fake managed lease has no localization-edit reader');
    }
    return read(
      this,
      localizationId,
      expectedLocalizationRevision,
      expectedLocId,
    );
  }

  @override
  Future<Revision3DialogLocalizationEditPublication>
  prepareAndPublishDialogLocalizationEditV1({
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  }) async {
    dialogLocalizationEditPublishCalls++;
    final publish = onDialogLocalizationEditPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no localization-edit publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  }) async {
    questSourceInspectionCalls++;
    final inspect = onQuestSourceInspection;
    if (inspect == null) {
      throw UnimplementedError(
        'fake managed lease has no Quest source inspector',
      );
    }
    return inspect(this, gameRoot, questId);
  }

  @override
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  }) async {
    npcSourceInspectionCalls++;
    npcSourceInspectionNpcIds.add(npcId);
    final inspect = onNpcSourceInspection;
    if (inspect == null) {
      throw StateError('fake managed lease has no NPC source inspector');
    }
    return inspect(this, npcId);
  }

  @override
  Future<ManagedRevision3CompilerCheckReceipt> checkCompilerV1({
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String gameRoot,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    managedCompilerCheckCalls++;
    final check = onManagedCompilerCheck;
    if (check == null) {
      throw StateError('fake managed lease has no managed compiler check');
    }
    return check(
      this,
      entityKind,
      gameRoot,
      entityId,
      expectedEntityRevision,
      expectedModuleId,
      expectedModuleRevision,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot}) async {
    dataAssetPackageIndexReadCalls++;
    final read = onDataAssetPackageIndexRead;
    if (read == null) {
      throw StateError('fake managed lease has no DataAsset package index');
    }
    return read(this, gameRoot);
  }

  @override
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) async {
    installedDataAssetInspectCalls++;
    final inspect = onInstalledDataAssetInspect;
    if (inspect == null) {
      throw StateError(
        'fake managed lease has no installed DataAsset inspector',
      );
    }
    return inspect(this, gameRoot, expectedSnapshot, candidate);
  }

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishInstalledDataAssetEditV1({
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  }) => throw StateError(
    'fake managed lease has no installed DataAsset edit publisher',
  );

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishReviewedInstalledDataAssetEditV1({
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) async {
    reviewedInstalledDataAssetPublishCalls++;
    final publish = onReviewedInstalledDataAssetPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no reviewed installed DataAsset edit publisher',
      );
    }
    return publish(this, gameRoot, intent);
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
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    questTransitionsSeedCalls++;
    final read = onQuestTransitionsSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest transitions seed');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
    );
  }

  @override
  Future<Revision3QuestTransitionsEditPublication>
  prepareAndPublishQuestTransitionsEditV1({
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  }) async {
    questTransitionsPublishCalls++;
    final publish = onQuestTransitionsPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest transitions publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) async {
    questContextSeedCalls++;
    final read = onQuestContextSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest context seed reader');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
      expectedParentRuntimeClass,
      expectedGiverRuntimeUniqueName,
    );
  }

  @override
  Future<Revision3QuestContextEditPublication>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  }) async {
    questContextPublishCalls++;
    final publish = onQuestContextPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest context publisher');
    }
    return publish(this, gameRoot, plan);
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
  Future<Revision3DialogLineEntryPublication> prepareAndPublishDialogLineV1({
    required Revision3DialogLineEntryTechnicalPlan plan,
  }) async {
    dialogLinePublishCalls++;
    final publish = onDialogLinePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no dialog-line publisher');
    }
    return publish(this, plan);
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
  Future<Revision3VoiceTakeSelectionPublication>
  prepareAndPublishVoiceTakeSelectionV1({
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  }) async {
    voiceSelectionPublishCalls++;
    final publish = onVoiceSelectionPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice selection publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<Revision3VoiceTakeStatusPublication>
  prepareAndPublishVoiceTakeStatusV1({
    required Revision3VoiceTakeStatusTechnicalPlan plan,
  }) async {
    voiceStatusPublishCalls++;
    final publish = onVoiceStatusPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice status publisher');
    }
    return publish(this, plan);
  }

  @override
  void markRequiresReopenAfterPublicationUncertainty() {
    requiresReopenValue = true;
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
  Future<AuthoringRevision3VoiceBuildPlanResult> planVoiceV1() async {
    voicePlanCalls++;
    final plan = onVoicePlan;
    if (plan == null) {
      throw StateError('fake managed lease has no Voice build planner');
    }
    return plan(this);
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

final class _FakeVoiceMediaQaManagedLease extends _FakeManagedLease
    implements ManagedRevision3VoiceTakeMediaQaLease {
  _FakeVoiceMediaQaManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required super.contentIndexBuilder,
  });

  int voiceMediaQaCalls = 0;
  Revision3VoiceTakePreviewTechnicalPlan? lastVoiceMediaQaPlan;

  @override
  bool get supportsVoiceTakeMediaQa => true;

  @override
  void markRequiresReopenAfterVoiceTakeMediaQaUncertainty() {
    requiresReopenValue = true;
  }

  @override
  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspectVoiceTakeMediaQaV1({
    required Revision3VoiceTakePreviewTechnicalPlan plan,
  }) async {
    voiceMediaQaCalls++;
    lastVoiceMediaQaPlan = plan;
    final request = AuthoringRevision3VoiceTakePreviewRequestV1(
      expectedHead: head,
      expectedProjectId: projectId,
      expectedRevision: projectRevision,
      lineId: plan.lineId,
      expectedLineRevision: plan.expectedLineRevision,
      localizationId: plan.localizationId,
      expectedLocalizationRevision: plan.expectedLocalizationRevision,
      expectedLocId: plan.locId,
      slotId: plan.slotId,
      expectedSlotRevision: plan.expectedSlotRevision,
      locale: plan.locale,
      takeId: plan.takeId,
      expectedTakeRevision: plan.expectedTakeRevision,
      expectedAsset: AuthoringRevision3VoiceTakePreviewExpectedAsset(
        sha256: plan.assetSha256,
        byteLength: plan.assetByteLength,
        logicalName: plan.assetLogicalName,
      ),
    );
    return AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
      revision3VoiceMediaQaResponse(request: request),
      request: request,
    );
  }
}

final class _FakeDialogVoiceSlotCreationManagedLease extends _FakeManagedLease
    implements ManagedRevision3DialogVoiceSlotCreationLease {
  _FakeDialogVoiceSlotCreationManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.onDialogVoiceSlotCreation,
    super.onDialogLocalizationRead,
    super.onDialogLocalizationEditSeed,
    super.contentIndexBuilder,
  });

  final _DialogVoiceSlotCreationCallback onDialogVoiceSlotCreation;
  int dialogVoiceSlotCreationCalls = 0;
  Revision3DialogVoiceSlotCreationTechnicalPlan? lastDialogVoiceSlotPlan;

  @override
  bool get supportsDialogVoiceSlotCreation => true;

  @override
  void markRequiresReopenAfterDialogVoiceSlotCreationUncertainty() {
    requiresReopenValue = true;
  }

  @override
  Future<Revision3DialogVoiceSlotCreationPublication>
  prepareAndPublishDialogVoiceSlotCreationV1({
    required Revision3DialogVoiceSlotCreationTechnicalPlan plan,
  }) async {
    dialogVoiceSlotCreationCalls++;
    lastDialogVoiceSlotPlan = plan;
    return onDialogVoiceSlotCreation(this, plan);
  }
}

final class _FakeQuestTranscriptManagedLease extends _FakeManagedLease
    implements ManagedRevision3QuestTranscriptLease {
  _FakeQuestTranscriptManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.onQuestTranscriptCreate,
    super.onQuestPublish,
    super.onDialogLocalizationRead,
    super.contentIndexBuilder,
  });

  final _QuestTranscriptCreateCallback onQuestTranscriptCreate;
  int questTranscriptCreateCalls = 0;

  @override
  bool get supportsQuestTranscript => true;

  @override
  void markRequiresReopenAfterQuestTranscriptUncertainty() {
    requiresReopenValue = true;
  }

  @override
  Future<Revision3QuestTranscriptPublication>
  prepareAndPublishQuestTranscriptCreateV1({
    required Revision3QuestTranscriptCreateTechnicalPlan plan,
  }) async {
    questTranscriptCreateCalls++;
    return onQuestTranscriptCreate(this, plan);
  }

  @override
  Future<Revision3QuestTranscriptPublication>
  prepareAndPublishQuestTranscriptReplaceV1({
    required Revision3QuestTranscriptReplaceTechnicalPlan plan,
  }) => throw StateError(
    'opening-recipe fake does not support transcript replacement',
  );
}

final class _FakeNpcGreetingManagedLease extends _FakeManagedLease
    implements ManagedRevision3NpcGreetingLease {
  _FakeNpcGreetingManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.onNpcGreetingCreate,
    super.onNpcPublish,
    super.onDialogLocalizationRead,
    super.contentIndexBuilder,
  });

  final _NpcGreetingCreateCallback onNpcGreetingCreate;
  int npcGreetingCreateCalls = 0;

  @override
  bool get supportsNpcGreeting => true;

  @override
  void markRequiresReopenAfterNpcGreetingUncertainty() {
    requiresReopenValue = true;
  }

  @override
  Future<Revision3NpcGreetingPublication> prepareAndPublishNpcGreetingCreateV1({
    required Revision3NpcGreetingCreateTechnicalPlan plan,
  }) async {
    npcGreetingCreateCalls++;
    return onNpcGreetingCreate(this, plan);
  }

  @override
  Future<Revision3NpcGreetingPublication>
  prepareAndPublishNpcGreetingReplaceV1({
    required Revision3NpcGreetingReplaceTechnicalPlan plan,
  }) => throw StateError(
    'opening-recipe fake does not support greeting replacement',
  );
}

typedef _HistoryRestoreCallback =
    FutureOr<ManagedRevision3ProjectHistoryRestoreCheckpoint> Function(
      _FakeHistoryManagedLease lease,
      Revision3ProjectHistorySnapshot expectedHistory,
      Revision3ProjectHistoryEntry target,
    );

final class _FakeHistoryManagedLease extends _FakeManagedLease
    implements ManagedRevision3ProjectHistoryLease {
  _FakeHistoryManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.history,
    super.canonicalProjectJsonValue,
    super.contentIndexBuilder,
    this.onRestore,
  });

  final Revision3ProjectHistorySnapshot history;
  final _HistoryRestoreCallback? onRestore;
  int historyReadCalls = 0;
  int historyRestoreCalls = 0;

  @override
  bool get supportsProjectHistory => true;

  @override
  Future<Revision3ProjectHistorySnapshot> readProjectHistoryV1() async {
    historyReadCalls++;
    return history;
  }

  @override
  Future<ManagedRevision3ProjectHistoryRestoreCheckpoint>
  prepareAndPublishProjectHistoryRestoreV1({
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  }) async {
    historyRestoreCalls++;
    final restore = onRestore;
    if (restore == null) {
      throw StateError('history restore is not configured for this fake');
    }
    return restore(this, expectedHistory, target);
  }

  @override
  void markRequiresReopenAfterHistoryUncertainty() {
    requiresReopenValue = true;
  }
}

final class _FakeExportManagedLease extends _FakeManagedLease
    implements ManagedRevision3RestorableProjectExportLease {
  _FakeExportManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required super.contentIndexBuilder,
    required this.onExport,
  });

  final _ProjectExportCallback onExport;
  int exportCalls = 0;

  @override
  bool get supportsRestorableSnapshotExport => true;

  @override
  Future<AuthoringRevision3ExactSnapshotExportResultV2> exportExactSnapshotV2({
    required String output,
  }) async {
    exportCalls++;
    return onExport(this, output);
  }
}

final class _FakeRecoverableManagedLease extends _FakeManagedLease
    implements ManagedRevision3RecoveryLease {
  _FakeRecoverableManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.onRecovery,
    super.canonicalProjectJsonValue,
    super.onQuestTransitionsSeed,
    super.contentIndexBuilder,
  });

  final _RecoveryCallback onRecovery;
  int recoveryCalls = 0;
  int recoveryUncertaintyMarks = 0;

  @override
  Future<ManagedRevision3RecoveryCheckpoint>
  recoverAfterUncertainPublication() async {
    recoveryCalls++;
    return onRecovery(this);
  }

  @override
  void markRequiresReopenAfterRecoveryUncertainty() {
    recoveryUncertaintyMarks++;
    requiresReopenValue = true;
  }
}

AuthoringRevision3DataAssetPackageIndexResult _homeDataAssetPackageIndexResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  String targetPath = '/Game/Characters/DA_Asghan',
}) {
  final packageIndexJson = jsonEncode(<String, Object?>{
    'status': 'complete_index',
    'physical_chunk_count': 1,
    'winning_export_bundle_count': 1,
    'directory_indexed_export_bundle_count': 1,
    'out_of_scope_export_bundle_count': 0,
    'candidates': <Object?>[
      <String, Object?>{
        'target_path': targetPath,
        'package_id_hex': '0123456789abcdef',
      },
    ],
    'partial_reasons': <Object?>[],
  });
  final packageIndexBytes = utf8.encode(packageIndexJson);
  Map<String, Object?> seal(int byteLength, String sha256) => <String, Object?>{
    'byte_len': byteLength,
    'sha256': sha256,
  };
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': 1,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': seal(80, 'b' * 64),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': packageIndexJson,
      'package_index_seal': seal(
        packageIndexBytes.length,
        crypto.sha256.convert(packageIndexBytes).toString(),
      ),
      'package_index_status': 'complete_index',
      'project_id': projectId,
      'project_revision': projectRevision,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': seal(120, 'c' * 64),
      'target_executable_seal': seal(1, '5' * 64),
    },
    expectedHead: head,
  );
}

const _homeReviewedWolfTargetPath =
    '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';

AuthoringRevision3InstalledDataAssetInspectionResult
_homeInstalledDataAssetInspectionResult({
  required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
  required AuthoringRevision3DataAssetPackageCandidate candidate,
  required Map<String, Object?> inspection,
}) => AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
  <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': candidate.ordinal,
    'head_json': expectedSnapshot.head.canonicalJson,
    'inspection': inspection,
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'inspection_only',
    'package_id_hex': candidate.packageIdHex,
    'package_index_seal': <String, Object?>{
      'byte_len': expectedSnapshot.packageIndexSeal.byteLength,
      'sha256': expectedSnapshot.packageIndexSeal.sha256,
    },
    'project_id': expectedSnapshot.projectId,
    'project_revision': expectedSnapshot.projectRevision,
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'selected_installed_dataasset_fixed_leaf_inspection_only',
    'source_snapshot_seal': <String, Object?>{
      'byte_len': expectedSnapshot.sourceSnapshotSeal.byteLength,
      'sha256': expectedSnapshot.sourceSnapshotSeal.sha256,
    },
    'target_path': candidate.targetPath,
    'usmap_content_seal': <String, Object?>{
      'byte_len': 256,
      'sha256': 'c' * 64,
    },
    'usmap_inventory_seal': <String, Object?>{
      'byte_len': 96,
      'sha256': 'e' * 64,
    },
  },
  expectedSnapshot: expectedSnapshot,
  requestedOrdinal: candidate.ordinal,
);

Map<String, Object?> _homeReviewedWolfInspectionResponse() {
  final response = validDataAssetInspectionResponse(
    objectName: 'DA_WolfFootsteps',
  );
  response['summary'] = <String, Object?>{
    'package_exports': 1,
    'reported_exports': 1,
    'walked_exports': 1,
    'editable_leaves': 1,
  };
  final export = ((response['exports'] as List).single as Map)
      .cast<String, Object?>();
  export
    ..['class_path'] = '/Script/G1R.FootstepTag'
    ..['schema'] = '/Script/G1R.FootstepTag'
    ..['leaves'] = <Object?>[
      <String, Object?>{
        'index': 0,
        'editable': true,
        'selector': _homeReviewedWolfSelector(),
      },
    ];
  return response;
}

Map<String, Object?> _homeReviewedWolfSelector() => <String, Object?>{
  'format': 1,
  'profile': 'g1r_ue5_4',
  'package_seal': <String, Object?>{
    'uasset_sha256': 'a' * 64,
    'uexp_sha256': 'b' * 64,
  },
  'usmap_sha256': 'c' * 64,
  'export_index': 0,
  'object_name': 'DA_WolfFootsteps',
  'class_path': '/Script/G1R.FootstepTag',
  'component': 'uexp',
  'export_sha256': 'd' * 64,
  'role': 'property_value',
  'kind': 'vector4_f64x4',
  'path': <Object?>[
    <String, Object?>{
      'step': 'property',
      'schema_index': 0,
      'property_name': 'BoneData',
      'array_index': 0,
      'array_dimension': 1,
      'declaring_schema_name': 'FootstepTag',
      'declaring_module_path': '/Script/G1R',
      'property_type': <String, Object?>{
        'type': 'struct',
        'name': 'BoneFeetData',
      },
    },
    <String, Object?>{
      'step': 'struct',
      'name': 'BoneFeetData',
      'schema_name': '/Script/G1R.BoneFeetData',
    },
    <String, Object?>{
      'step': 'property',
      'schema_index': 0,
      'property_name': 'FeetTextureSize',
      'array_index': 0,
      'array_dimension': 1,
      'declaring_schema_name': 'BoneFeetData',
      'declaring_module_path': '/Script/G1R',
      'property_type': <String, Object?>{'type': 'struct', 'name': 'Vector4'},
    },
  ],
  'expected_hex':
      '00000000000024400000000000002440'
      '0000000000000000000000000000f03f',
};

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

AuthoringRevision3ExactSnapshotExportResultV2 _homeProjectExportResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String output,
}) => AuthoringRevision3ExactSnapshotExportResultV2.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'exported',
    'format': 'managed_revision3_exact_snapshot_v2',
    'artifact_kind': 'portable_snapshot_restorable_copy',
    'restore_status': 'supported',
    'basis_head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': projectRevision,
    'output': output,
    'archive': <String, Object?>{'byte_len': 300, 'sha256': 'a' * 64},
    'manifest': <String, Object?>{
      'relative_name': 'gore-export.json',
      'byte_len': 100,
      'sha256': 'b' * 64,
    },
    'closure': <String, Object?>{
      'snapshot_objects': 1,
      'entity_objects': 0,
      'asset_objects': 0,
      'archive_entries': 4,
      'uncompressed_bytes': 200,
    },
    'publication_status': 'published',
    'retry_safe': false,
    'warning': null,
    'project_mutation': 'not_performed',
    'game_mutation': 'not_performed',
    'save_mutation': 'not_performed',
    'build_status': 'not_performed',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
  },
  expectedHead: head,
  expectedOutput: output,
);

Map<String, Object?> _homeProjectImportInspectionResponse({
  required String source,
  required String projectId,
  required int projectRevision,
  required AuthoringWorkingHead head,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'inspected_restorable_copy',
  'source': source,
  'format': revision3ProjectImportFormatV2,
  'artifact_kind': revision3ProjectImportArtifactKindV2,
  'restore_status': revision3ProjectImportRestoreStatusV2,
  'archive': <String, Object?>{'byte_len': 8192, 'sha256': 'a' * 64},
  'manifest': <String, Object?>{
    'relative_name': revision3ProjectImportManifestName,
    'byte_len': 512,
    'sha256': 'b' * 64,
  },
  'project_id': projectId,
  'project_revision': projectRevision,
  'head_json': head.canonicalJson,
  'closure': <String, Object?>{
    'snapshot_objects': 1,
    'entity_objects': 1,
    'asset_objects': 1,
    'archive_entries': 6,
    'uncompressed_bytes': 4096,
  },
  'inspection_status': 'verified_exact',
  'import_status': 'not_performed',
  'project_mutation': 'not_performed',
  'game_mutation': 'not_performed',
  'save_mutation': 'not_performed',
  'build_status': 'not_performed',
  'deployment_status': 'not_performed',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_supported',
  'retry_safe': true,
};

Map<String, Object?> _homeProjectImportDestinationResponse({
  required Revision3ProjectImportDestinationRequest request,
  required String projectId,
  required int projectRevision,
  required AuthoringWorkingHead head,
  required String outcome,
}) {
  final response = <String, Object?>{
    'ok': true,
    'outcome': outcome,
    'source': request.source,
    'destination': request.destination,
    'format': revision3ProjectImportFormatV2,
    'artifact_kind': revision3ProjectImportArtifactKindV2,
    'restore_status': revision3ProjectImportRestoreStatusV2,
    'inspection_status': 'verified_exact',
    'import_status': 'materialized',
    'project_mutation': 'materialized',
    'session_adoption': 'not_performed',
    'game_mutation': 'not_performed',
    'save_mutation': 'not_performed',
    'build_status': 'not_performed',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'publication_status': switch (outcome) {
      'imported' => 'published',
      'imported_with_cleanup_warning' => 'published_with_cleanup_warning',
      'publication_uncertain' => 'publication_uncertain',
      _ => 'published',
    },
    'retry_safe': false,
    'warning': switch (outcome) {
      'imported' => null,
      'imported_with_cleanup_warning' => <String, Object?>{
        'code': 'AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING',
        'message':
            'the verified project was materialized, but private staging cleanup was incomplete',
      },
      'publication_uncertain' => <String, Object?>{
        'code': 'AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN',
        'message':
            'project publication may have completed; do not retry automatically',
      },
      _ => null,
    },
  };
  if (outcome != 'publication_uncertain') {
    response.addAll(<String, Object?>{
      'archive': <String, Object?>{
        'byte_len': request.expectedArchive.byteLength,
        'sha256': request.expectedArchive.sha256,
      },
      'manifest': <String, Object?>{
        'relative_name': revision3ProjectImportManifestName,
        'byte_len': 512,
        'sha256': 'b' * 64,
      },
      'project_id': projectId,
      'project_revision': projectRevision,
      'head_json': head.canonicalJson,
      'closure': <String, Object?>{
        'snapshot_objects': 1,
        'entity_objects': 1,
        'asset_objects': 1,
        'archive_entries': 6,
        'uncompressed_bytes': 4096,
      },
    });
  }
  return response;
}

({String projectJson, AuthoringWorkingHead head}) _recoverySnapshot({
  required String projectId,
  required int revision,
}) {
  final projectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': projectId,
    'revision': revision,
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256':
            'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
      },
    },
    'entities': <String, Object?>{},
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
  final bytes = utf8.encode(projectJson);
  return (
    projectJson: projectJson,
    head: AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': bytes.length,
          'sha256': crypto.sha256.convert(bytes).toString(),
        },
      }),
    ),
  );
}

ManagedRevision3RecoveryCheckpoint _recoveryCheckpoint({
  required String projectId,
  required int previousRevision,
  required AuthoringWorkingHead previousHead,
  required int recoveredRevision,
  required AuthoringWorkingHead recoveredHead,
  required String recoveredProjectJson,
}) => ManagedRevision3RecoveryCheckpoint(
  previousHead: previousHead,
  recoveredHead: recoveredHead,
  projectId: projectId,
  previousProjectRevision: previousRevision,
  recoveredProjectRevision: recoveredRevision,
  repairOutcome: AtomicRepairOutcome.clean,
  canonicalProjectJson: recoveredProjectJson,
);

Revision3ContentIndex _contentIndex({
  required String projectId,
  required int revision,
  String projectName = 'Home Quest project',
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': projectName,
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

Revision3ContentIndex _createdNpcContentIndex({
  required String projectId,
  required int revision,
  required String npcId,
  required String moduleId,
  required String displayName,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Home NPC project',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>['de'],
  'entity_counts': <String, Object?>{'npc_draft': 1, 'script_module': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': npcId,
      'kind': 'npc_draft',
      'display_name': displayName,
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': _homeCreatedNpcUniqueName,
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': _homeCreatedNpcUniqueName,
          'module_namespace': _homeCreatedNpcModuleNamespace,
          'parent_character_definition':
              'UCharacterDefinition_Human_OM_GRD_Asghan_263',
          'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
          'parent_spawn_definition':
              'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
          'greeting_count': 0,
        },
      },
      'references': <Object?>[
        _dialogLineEntityReference(
          projectId: projectId,
          role: 'draft_script_module',
          entityId: moduleId,
          expectedKind: 'script_module',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': moduleId,
      'kind': 'script_module',
      'display_name': 'North Gate Guard Script',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': revision3NpcFixtureGeneratorId,
        'generator_version': revision3NpcFixtureGeneratorVersion,
        'owner': <String, Object?>{
          'project_id': projectId,
          'entity_id': npcId,
          'expected_kind': 'npc_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': revision3NpcFixtureGeneratorId,
          'generator_version': revision3NpcFixtureGeneratorVersion,
          'module_namespace': _homeCreatedNpcModuleNamespace,
          'module_relative_path': 'GoreMods/Npcs/NorthGateGuard.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _dialogLineEntityReference(
          projectId: projectId,
          role: 'origin_owner',
          entityId: npcId,
          expectedKind: 'npc_draft',
        ),
        _dialogLineEntityReference(
          projectId: projectId,
          role: 'script_owner',
          entityId: npcId,
          expectedKind: 'npc_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

Revision3ContentIndex _npcOpeningRecipeContentIndex({
  required String projectId,
  required int projectRevision,
  required int openingRevision,
  Revision3NpcGreetingCreateTechnicalPlan? createPlan,
}) {
  if (projectRevision == openingRevision) {
    return _contentIndex(projectId: projectId, revision: projectRevision);
  }
  final npcOnlyRevision = openingRevision + 1;
  final completedRevision = openingRevision + 2;
  if (projectRevision != npcOnlyRevision &&
      projectRevision != completedRevision) {
    throw StateError(
      'unexpected Character-opening fixture revision $projectRevision',
    );
  }
  final completed = projectRevision == completedRevision;
  if (completed && createPlan == null) {
    throw StateError(
      'N+2 Character-opening fixture requires the published plan',
    );
  }
  final line = completed ? createPlan!.line : null;
  AuthoringRevision3DialogLocalizationCreateIntentV1? localization;
  if (line != null) {
    final intent = line.localization;
    if (intent is! AuthoringRevision3DialogLocalizationCreateIntentV1) {
      throw StateError(
        'Character-opening fixture requires created localization',
      );
    }
    localization = intent;
  }
  final slot = line?.voiceSlot;
  final entities =
      <Map<String, Object?>>[
        <String, Object?>{
          'id': _homeCreatedNpcId,
          'kind': 'npc_draft',
          'display_name': 'North Gate Guard',
          'revision': completed ? createPlan!.expectedNpcRevision + 1 : 0,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': _homeCreatedNpcUniqueName,
          },
          'summary': <String, Object?>{
            'kind': 'npc_draft',
            'data': <String, Object?>{
              'unique_name': _homeCreatedNpcUniqueName,
              'module_namespace': _homeCreatedNpcModuleNamespace,
              'parent_character_definition':
                  'UCharacterDefinition_Human_OM_GRD_Asghan_263',
              'parent_ai_agent_config':
                  'UAIAgentConfig_Human_OM_GRD_Asghan_263',
              'parent_spawn_definition':
                  'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
              'greeting_count': completed ? 1 : 0,
            },
          },
          'references': <Object?>[
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'draft_script_module',
              entityId: _homeCreatedNpcModuleId,
              expectedKind: 'script_module',
            ),
            if (line != null)
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'npc_greeting_line',
                entityId: line.lineId,
                expectedKind: 'dialog_line',
              ),
          ],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': _homeCreatedNpcModuleId,
          'kind': 'script_module',
          'display_name': 'North Gate Guard Script',
          'revision': 0,
          'origin': <String, Object?>{
            'type': 'generated',
            'generator_id': revision3NpcFixtureGeneratorId,
            'generator_version': revision3NpcFixtureGeneratorVersion,
            'owner': <String, Object?>{
              'project_id': projectId,
              'entity_id': _homeCreatedNpcId,
              'expected_kind': 'npc_draft',
            },
          },
          'summary': <String, Object?>{
            'kind': 'script_module',
            'data': <String, Object?>{
              'generator_id': revision3NpcFixtureGeneratorId,
              'generator_version': revision3NpcFixtureGeneratorVersion,
              'module_namespace': _homeCreatedNpcModuleNamespace,
              'module_relative_path': 'GoreMods/Npcs/NorthGateGuard.as',
              'status': <String, Object?>{
                'authoring': 'offline_draft',
                'runtime': 'runtime_unqualified',
              },
            },
          },
          'references': <Object?>[
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'origin_owner',
              entityId: _homeCreatedNpcId,
              expectedKind: 'npc_draft',
            ),
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'script_owner',
              entityId: _homeCreatedNpcId,
              expectedKind: 'npc_draft',
            ),
          ],
          'asset_references': <Object?>[],
        },
        if (localization != null)
          <String, Object?>{
            'id': localization.localizationId,
            'kind': 'localization_entry',
            'display_name': localization.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': localization.locId,
            },
            'summary': <String, Object?>{
              'kind': 'localization_entry',
              'data': <String, Object?>{
                'loc_id': localization.locId,
                'locales': <Object?>[...localization.texts.keys],
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
        if (line != null)
          <String, Object?>{
            'id': line.lineId,
            'kind': 'dialog_line',
            'display_name': line.lineDisplayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': line.lineAuthoredIdentity,
            },
            'summary': <String, Object?>{
              'kind': 'dialog_line',
              'data': <String, Object?>{
                'speaker_hint': line.speakerHint,
                'voice_slot_locales': <Object?>[if (slot != null) slot.locale],
              },
            },
            'references': <Object?>[
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'dialog_localization',
                entityId: localization!.localizationId,
                expectedKind: 'localization_entry',
              ),
              if (slot != null)
                _dialogLineEntityReference(
                  projectId: projectId,
                  role: 'dialog_voice_slot',
                  qualifier: slot.locale,
                  entityId: slot.slotId,
                  expectedKind: 'voice_slot',
                ),
            ],
            'asset_references': <Object?>[],
          },
        if (slot != null)
          <String, Object?>{
            'id': slot.slotId,
            'kind': 'voice_slot',
            'display_name': slot.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'GORE_VOICE_${slot.slotId.toUpperCase()}',
            },
            'summary': <String, Object?>{
              'kind': 'voice_slot',
              'data': <String, Object?>{
                'locale': slot.locale,
                'target_resolution': 'unresolved',
                'candidate_count': 0,
                'has_selected_take': false,
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': projectId,
    'project_revision': projectRevision,
    'project_name': 'Home Character opening recipe project',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 1,
        'sha256': List<String>.filled(64, '5').join(),
      },
    },
    'authoring_locales': <Object?>['de', 'en'],
    'entity_counts': <String, Object?>{
      if (localization != null) 'localization_entry': 1,
      if (line != null) 'dialog_line': 1,
      if (slot != null) 'voice_slot': 1,
      'npc_draft': 1,
      'script_module': 1,
    },
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3ContentIndex _questOpeningRecipeContentIndex({
  required String projectId,
  required int projectRevision,
  required int openingRevision,
  Revision3QuestTranscriptCreateTechnicalPlan? createPlan,
}) {
  if (projectRevision == openingRevision) {
    return _contentIndex(projectId: projectId, revision: projectRevision);
  }
  final questOnlyRevision = openingRevision + 1;
  final completedRevision = openingRevision + 2;
  if (projectRevision != questOnlyRevision &&
      projectRevision != completedRevision) {
    throw StateError(
      'unexpected Quest-opening fixture revision $projectRevision',
    );
  }
  final completed = projectRevision == completedRevision;
  if (completed && createPlan == null) {
    throw StateError('N+2 Quest-opening fixture requires the published plan');
  }
  final line = completed ? createPlan!.line : null;
  AuthoringRevision3DialogLocalizationCreateIntentV1? localization;
  if (line != null) {
    final intent = line.localization;
    if (intent is! AuthoringRevision3DialogLocalizationCreateIntentV1) {
      throw StateError('Quest-opening fixture requires created localization');
    }
    localization = intent;
  }
  final slot = line?.voiceSlot;
  final questRevision = completed ? createPlan!.expectedQuestRevision + 1 : 4;
  final moduleRevision = completed ? createPlan!.expectedModuleRevision : 5;
  final entities =
      <Map<String, Object?>>[
        <String, Object?>{
          'id': _homeQuestOpeningRecipeQuestId,
          'kind': 'quest_draft',
          'display_name': 'Warn Asghan',
          'revision': questRevision,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': _homeQuestOpeningRecipeTechnicalId,
          },
          'summary': <String, Object?>{
            'kind': 'quest_draft',
            'data': <String, Object?>{
              'technical_id': _homeQuestOpeningRecipeTechnicalId,
              'title': 'Warn Asghan',
              'objective_title': 'Listen to Asghan',
              'objective_slots': <Object?>[1],
              'transcript_count': completed ? 1 : 0,
              'module_namespace': 'PROJECT.QUESTS.WARNASGHAN',
              'parent_runtime_class': 'B_Quest_FindHomer_C',
              'giver_runtime_unique_name': 'ASGHAN',
            },
          },
          'references': <Object?>[
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'draft_script_module',
              entityId: _homeQuestOpeningRecipeModuleId,
              expectedKind: 'script_module',
            ),
            if (line != null)
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'quest_transcript_line',
                qualifier: createPlan!.objectiveSlot?.toString(),
                entityId: line.lineId,
                expectedKind: 'dialog_line',
              ),
          ],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': _homeQuestOpeningRecipeModuleId,
          'kind': 'script_module',
          'display_name': 'Warn Asghan Script',
          'revision': moduleRevision,
          'origin': <String, Object?>{
            'type': 'generated',
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': 4,
            'owner': <String, Object?>{
              'project_id': projectId,
              'entity_id': _homeQuestOpeningRecipeQuestId,
              'expected_kind': 'quest_draft',
            },
          },
          'summary': <String, Object?>{
            'kind': 'script_module',
            'data': <String, Object?>{
              'generator_id': 'gore-authoring.draft-quest-skeleton',
              'generator_version': 4,
              'module_namespace': 'PROJECT.QUESTS.WARNASGHAN',
              'module_relative_path': 'PROJECT/QUESTS/WARNASGHAN.as',
              'status': <String, Object?>{
                'authoring': 'offline_draft',
                'runtime': 'runtime_unqualified',
              },
            },
          },
          'references': <Object?>[
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'origin_owner',
              entityId: _homeQuestOpeningRecipeQuestId,
              expectedKind: 'quest_draft',
            ),
          ],
          'asset_references': <Object?>[],
        },
        if (localization != null)
          <String, Object?>{
            'id': localization.localizationId,
            'kind': 'localization_entry',
            'display_name': localization.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': localization.locId,
            },
            'summary': <String, Object?>{
              'kind': 'localization_entry',
              'data': <String, Object?>{
                'loc_id': localization.locId,
                'locales': <Object?>[...localization.texts.keys],
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
        if (line != null)
          <String, Object?>{
            'id': line.lineId,
            'kind': 'dialog_line',
            'display_name': line.lineDisplayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': line.lineAuthoredIdentity,
            },
            'summary': <String, Object?>{
              'kind': 'dialog_line',
              'data': <String, Object?>{
                'speaker_hint': line.speakerHint,
                'voice_slot_locales': <Object?>[if (slot != null) slot.locale],
              },
            },
            'references': <Object?>[
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'dialog_localization',
                entityId: localization!.localizationId,
                expectedKind: 'localization_entry',
              ),
              if (slot != null)
                _dialogLineEntityReference(
                  projectId: projectId,
                  role: 'dialog_voice_slot',
                  qualifier: slot.locale,
                  entityId: slot.slotId,
                  expectedKind: 'voice_slot',
                ),
            ],
            'asset_references': <Object?>[],
          },
        if (slot != null)
          <String, Object?>{
            'id': slot.slotId,
            'kind': 'voice_slot',
            'display_name': slot.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'GORE_VOICE_${slot.slotId.toUpperCase()}',
            },
            'summary': <String, Object?>{
              'kind': 'voice_slot',
              'data': <String, Object?>{
                'locale': slot.locale,
                'target_resolution': 'unresolved',
                'candidate_count': 0,
                'has_selected_take': false,
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': projectId,
    'project_revision': projectRevision,
    'project_name': 'Home Quest opening recipe project',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 1,
        'sha256': List<String>.filled(64, '5').join(),
      },
    },
    'authoring_locales': <Object?>['de', 'en'],
    'entity_counts': <String, Object?>{
      if (localization != null) 'localization_entry': 1,
      if (line != null) 'dialog_line': 1,
      if (slot != null) 'voice_slot': 1,
      'quest_draft': 1,
      'script_module': 1,
    },
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3ContentIndex _voiceLocalizationWorkspaceIndex({
  required int revision,
  int localizationRevision = 0,
  bool existingDeSlot = true,
  int existingSlotCandidateCount = 0,
  bool existingSlotHasSelectedTake = false,
  String existingSlotTargetResolution = 'unresolved',
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingDeSlot,
    existingSlotCandidateCount: existingSlotCandidateCount,
    existingSlotHasSelectedTake: existingSlotHasSelectedTake,
    existingSlotTargetResolution: existingSlotTargetResolution,
  );
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  localization['revision'] = localizationRevision;
  final summary = (localization['summary']! as Map).cast<String, Object?>();
  final data = (summary['data']! as Map).cast<String, Object?>();
  data['locales'] = <Object?>['de', 'en'];
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _dialogLocalizationEditIndex({
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': projectRevision,
  'project_name': 'Home localization edit',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{'localization_entry': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': localizationId,
      'kind': 'localization_entry',
      'display_name': 'Asghan warning',
      'revision': localizationRevision,
      'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': locId},
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': locId,
          'locales': <Object?>['de', 'en'],
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

AuthoringRevision3DialogLocalizationEditSeed _dialogLocalizationEditSeed({
  required _FakeManagedLease lease,
  required String localizationId,
  required int localizationRevision,
  required String locId,
  String? lineId,
  String lineDisplayName = 'Dialog line',
  String? speaker,
  Set<String> voiceSlotLocales = const <String>{},
  String germanText = 'Bleib stehen!',
  String englishText = 'Stop right there!',
}) {
  final request = AuthoringRevision3DialogLocalizationEditSeedRequestV1(
    expectedHead: lease.head,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationEditSeed.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': lease.head.canonicalJson,
      'project_id': lease.projectId,
      'project_revision': lease.projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': 'de',
          'text': germanText,
          'voice_slot_present': voiceSlotLocales.contains('de'),
          'candidate_count': 0,
        },
        <String, Object?>{
          'locale': 'en',
          'text': englishText,
          'voice_slot_present': voiceSlotLocales.contains('en'),
          'candidate_count': 0,
        },
      ],
      'line_backlinks': <Object?>[
        if (lineId != null)
          <String, Object?>{
            'line_id': lineId,
            'line_revision': 2,
            'display_name': lineDisplayName,
            'speaker_hint': speaker,
            'voice_slot_locales': voiceSlotLocales.toList()..sort(),
          },
      ],
      'content_authority': 'read_only_exact_current_localization_edit_seed',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

AuthoringRevision3DialogLocalizationReadResult _dialogLocalizationReadResult({
  required _FakeManagedLease lease,
  required String localizationId,
  required int localizationRevision,
  required String locId,
  required String nonemptyPreview,
}) {
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: lease.head,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': lease.head.canonicalJson,
      'project_id': lease.projectId,
      'project_revision': lease.projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': 'de',
          'preview': nonemptyPreview,
          'truncated': false,
          'has_nonempty_text': true,
        },
        <String, Object?>{
          'locale': 'en',
          'preview': '   ',
          'truncated': false,
          'has_nonempty_text': false,
        },
      ],
      'content_authority': 'read_only_exact_current_localization',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

AuthoringRevision3DialogLocalizationReadResult
_questOpeningRecipeLocalizationReadResult({
  required _FakeManagedLease lease,
  required Revision3DialogLineEntryTechnicalPlan line,
}) {
  final localization =
      line.localization as AuthoringRevision3DialogLocalizationCreateIntentV1;
  final text = localization.texts[line.locale];
  if (text == null) {
    throw StateError('opening-line localization has no selected locale');
  }
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: lease.head,
    localizationId: localization.localizationId,
    expectedLocalizationRevision: 0,
    expectedLocId: localization.locId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': lease.head.canonicalJson,
      'project_id': lease.projectId,
      'project_revision': lease.projectRevision,
      'localization_id': localization.localizationId,
      'localization_revision': 0,
      'loc_id': localization.locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': line.locale,
          'preview': text,
          'truncated': false,
          'has_nonempty_text': true,
        },
      ],
      'content_authority': 'read_only_exact_current_localization',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

Revision3ContentIndex _dialogReuseContentIndex({
  required String projectId,
  required int revision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
  required String displayName,
  required Revision3DialogLineEntryTechnicalPlan? publishedPlan,
}) {
  final plan = publishedPlan;
  final slot = plan?.voiceSlot;
  final entities =
      <Map<String, Object?>>[
        <String, Object?>{
          'id': localizationId,
          'kind': 'localization_entry',
          'display_name': displayName,
          'revision': localizationRevision,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': locId,
          },
          'summary': <String, Object?>{
            'kind': 'localization_entry',
            'data': <String, Object?>{
              'loc_id': locId,
              'locales': <Object?>['de', 'en'],
            },
          },
          'references': <Object?>[],
          'asset_references': <Object?>[],
        },
        if (plan != null)
          <String, Object?>{
            'id': plan.lineId,
            'kind': 'dialog_line',
            'display_name': plan.lineDisplayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': plan.lineAuthoredIdentity,
            },
            'summary': <String, Object?>{
              'kind': 'dialog_line',
              'data': <String, Object?>{
                'speaker_hint': plan.speakerHint,
                'voice_slot_locales': <Object?>[if (slot != null) slot.locale],
              },
            },
            'references': <Object?>[
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'dialog_localization',
                entityId: localizationId,
                expectedKind: 'localization_entry',
              ),
              if (slot != null)
                _dialogLineEntityReference(
                  projectId: projectId,
                  role: 'dialog_voice_slot',
                  qualifier: slot.locale,
                  entityId: slot.slotId,
                  expectedKind: 'voice_slot',
                ),
            ],
            'asset_references': <Object?>[],
          },
        if (slot != null)
          <String, Object?>{
            'id': slot.slotId,
            'kind': 'voice_slot',
            'display_name': slot.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'GORE_VOICE_${slot.slotId.toUpperCase()}',
            },
            'summary': <String, Object?>{
              'kind': 'voice_slot',
              'data': <String, Object?>{
                'locale': slot.locale,
                'target_resolution': 'unresolved',
                'candidate_count': 0,
                'has_selected_take': false,
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': projectId,
    'project_revision': revision,
    'project_name': 'Home Dialog reuse project',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 1,
        'sha256': List<String>.filled(64, '5').join(),
      },
    },
    'authoring_locales': <Object?>['de', 'en'],
    'entity_counts': <String, Object?>{
      'localization_entry': 1,
      if (plan != null) 'dialog_line': 1,
      if (slot != null) 'voice_slot': 1,
    },
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3ContentIndex _dialogLineContentIndex({
  required String projectId,
  required int revision,
  required Revision3DialogLineEntryTechnicalPlan plan,
}) {
  final localization =
      plan.localization as AuthoringRevision3DialogLocalizationCreateIntentV1;
  final slot = plan.voiceSlot;
  final entities =
      <Map<String, Object?>>[
        <String, Object?>{
          'id': localization.localizationId,
          'kind': 'localization_entry',
          'display_name': localization.displayName,
          'revision': 0,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': localization.locId,
          },
          'summary': <String, Object?>{
            'kind': 'localization_entry',
            'data': <String, Object?>{
              'loc_id': localization.locId,
              'locales': <Object?>[...localization.texts.keys],
            },
          },
          'references': <Object?>[],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': plan.lineId,
          'kind': 'dialog_line',
          'display_name': plan.lineDisplayName,
          'revision': 0,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': plan.lineAuthoredIdentity,
          },
          'summary': <String, Object?>{
            'kind': 'dialog_line',
            'data': <String, Object?>{
              'speaker_hint': plan.speakerHint,
              'voice_slot_locales': <Object?>[if (slot != null) slot.locale],
            },
          },
          'references': <Object?>[
            _dialogLineEntityReference(
              projectId: projectId,
              role: 'dialog_localization',
              entityId: localization.localizationId,
              expectedKind: 'localization_entry',
            ),
            if (slot != null)
              _dialogLineEntityReference(
                projectId: projectId,
                role: 'dialog_voice_slot',
                qualifier: slot.locale,
                entityId: slot.slotId,
                expectedKind: 'voice_slot',
              ),
          ],
          'asset_references': <Object?>[],
        },
        if (slot != null)
          <String, Object?>{
            'id': slot.slotId,
            'kind': 'voice_slot',
            'display_name': slot.displayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'GORE_VOICE_${slot.slotId.toUpperCase()}',
            },
            'summary': <String, Object?>{
              'kind': 'voice_slot',
              'data': <String, Object?>{
                'locale': slot.locale,
                'target_resolution': 'unresolved',
                'candidate_count': 0,
                'has_selected_take': false,
              },
            },
            'references': <Object?>[],
            'asset_references': <Object?>[],
          },
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': projectId,
    'project_revision': revision,
    'project_name': 'Home Dialog project',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 1,
        'sha256': List<String>.filled(64, '5').join(),
      },
    },
    'authoring_locales': <Object?>[...localization.texts.keys],
    'entity_counts': <String, Object?>{
      'localization_entry': 1,
      'dialog_line': 1,
      if (slot != null) 'voice_slot': 1,
    },
    'entities': entities,
    'assets': <Object?>[],
  });
}

Map<String, Object?> _dialogLineEntityReference({
  required String projectId,
  required String role,
  String? qualifier,
  required String entityId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': projectId,
    'entity_id': entityId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Revision3ContentIndex _globalSearchContentIndex({
  required String projectId,
  required int revision,
  required String targetEntityId,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Global search project',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{'npc_draft': 2},
  'entities': <Object?>[
    <String, Object?>{
      'id': '91919191919191919191919191919191',
      'kind': 'npc_draft',
      'display_name': 'First Sentinel',
      'revision': 1,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'FIRST_SENTINEL',
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': 'FIRST_SENTINEL',
          'module_namespace': 'Test_Global_Search',
          'parent_character_definition':
              'UCharacterDefinition_Human_OM_GRD_Gardist_261',
          'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Gardist_261',
          'parent_spawn_definition':
              'USpawnAIAgentDefinition_OM_GRD_Gardist_261',
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': targetEntityId,
      'kind': 'npc_draft',
      'display_name': 'Asghan Sentinel',
      'revision': 1,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'ASGHAN_SENTINEL',
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': 'ASGHAN_SENTINEL',
          'module_namespace': 'Test_Global_Search',
          'parent_character_definition':
              'UCharacterDefinition_Human_OM_GRD_Asghan_263',
          'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
          'parent_spawn_definition':
              'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

({
  String projectJson,
  AuthoringWorkingHead head,
  Revision3ContentIndex contentIndex,
  String sourceSha256,
})
_npcManagedCompilerFixture() {
  const revision = 7;
  final baseProjectJson = revision3NpcInspectionProjectJson(
    projectId: revision3NpcInspectionProjectId,
    revision: revision,
  );
  final baseProject = (jsonDecode(baseProjectJson) as Map)
      .cast<String, Object?>();
  final request = AuthoringRevision3NpcDraftRequestV1.forProject(
    expectedHead: revision3NpcFixtureHead(baseProjectJson),
    currentProjectJson: baseProjectJson,
    npcId: revision3NpcInspectionNpcId,
    scriptModuleId: revision3NpcInspectionModuleId,
    displayName: 'Inspection Guard',
    intent: AuthoringRevision3NpcDraftIntentV1(
      moduleNamespace: revision3NpcInspectionModuleNamespace,
      uniqueName: revision3NpcInspectionUniqueName,
      parentCatalogId: 'g1r:npc:om_grd_asghan_263',
    ),
  );
  final target = (baseProject['target']! as Map).cast<String, Object?>();
  final input = revision3NpcFixtureInput(request: request, target: target);
  final npcEntity = revision3NpcFixtureEntity(
    projectId: revision3NpcInspectionProjectId,
    request: request,
    input: input,
  )..['revision'] = 2;
  final moduleEntity = revision3NpcFixtureModuleEntity(
    projectId: revision3NpcInspectionProjectId,
    request: request,
    input: input,
  )..['revision'] = 3;
  baseProject['entities'] = <String, Object?>{
    revision3NpcInspectionNpcId: npcEntity,
    revision3NpcInspectionModuleId: moduleEntity,
  };
  final projectJson = jsonEncode(baseProject);
  final head = revision3NpcFixtureHead(projectJson);
  final modulePayload = (moduleEntity['payload']! as Map)
      .cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  final sourceSha256 = moduleData['source_sha256']! as String;
  final referenceToModule = <String, Object?>{
    'role': 'draft_script_module',
    'qualifier': null,
    'target': <String, Object?>{
      'project_id': revision3NpcInspectionProjectId,
      'entity_id': revision3NpcInspectionModuleId,
      'expected_kind': 'script_module',
    },
    'resolution': 'resolved',
  };
  final referenceToNpc = <String, Object?>{
    'role': 'origin_owner',
    'qualifier': null,
    'target': <String, Object?>{
      'project_id': revision3NpcInspectionProjectId,
      'entity_id': revision3NpcInspectionNpcId,
      'expected_kind': 'npc_draft',
    },
    'resolution': 'resolved',
  };
  final contentIndex = Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': revision3NpcInspectionProjectId,
    'project_revision': revision,
    'project_name': 'Home NPC compiler project',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': target,
    'authoring_locales': <Object?>[],
    'entity_counts': <String, Object?>{'npc_draft': 1, 'script_module': 1},
    'entities': <Object?>[
      <String, Object?>{
        'id': revision3NpcInspectionNpcId,
        'kind': 'npc_draft',
        'display_name': 'Inspection Guard',
        'revision': 2,
        'origin': npcEntity['origin'],
        'summary': <String, Object?>{
          'kind': 'npc_draft',
          'data': <String, Object?>{
            'unique_name': revision3NpcInspectionUniqueName,
            'module_namespace': revision3NpcInspectionModuleNamespace,
            'parent_character_definition':
                'UCharacterDefinition_Human_OM_GRD_Asghan_263',
            'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
            'parent_spawn_definition':
                'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
          },
        },
        'references': <Object?>[referenceToModule],
        'asset_references': <Object?>[],
      },
      <String, Object?>{
        'id': revision3NpcInspectionModuleId,
        'kind': 'script_module',
        'display_name': revision3NpcInspectionModuleNamespace,
        'revision': 3,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': revision3NpcFixtureGeneratorId,
          'generator_version': revision3NpcFixtureGeneratorVersion,
          'owner': <String, Object?>{
            'project_id': revision3NpcInspectionProjectId,
            'entity_id': revision3NpcInspectionNpcId,
            'expected_kind': 'npc_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': revision3NpcFixtureGeneratorId,
            'generator_version': revision3NpcFixtureGeneratorVersion,
            'module_namespace': revision3NpcInspectionModuleNamespace,
            'module_relative_path':
                '${revision3NpcInspectionModuleNamespace.replaceAll('.', '/')}.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[referenceToNpc],
        'asset_references': <Object?>[],
      },
    ],
    'assets': <Object?>[],
  });
  return (
    projectJson: projectJson,
    head: head,
    contentIndex: contentIndex,
    sourceSha256: sourceSha256,
  );
}

ManagedRevision3CompilerCheckReceipt _npcManagedCompilerReceipt({
  required String projectJson,
  required AuthoringWorkingHead head,
  required String sourceSha256,
}) {
  final bytes = utf8.encode(projectJson);
  final result = AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'compiler_check_only',
      'exact_current': true,
      'head_json': head.canonicalJson,
      'project': <String, Object?>{
        'id': revision3NpcInspectionProjectId,
        'revision': 7,
        'seal': <String, Object?>{
          'byte_len': bytes.length,
          'sha256': crypto.sha256.convert(bytes).toString(),
        },
      },
      'entity': <String, Object?>{
        'kind': 'npc_draft',
        'id': revision3NpcInspectionNpcId,
        'revision': 2,
      },
      'module': <String, Object?>{
        'id': revision3NpcInspectionModuleId,
        'revision': 3,
        'namespace': revision3NpcInspectionModuleNamespace,
        'relative_path':
            '${revision3NpcInspectionModuleNamespace.replaceAll('.', '/')}.as',
        'source_sha256': sourceSha256,
      },
      'compiler': <String, Object?>{
        'outcome': 'compiled_evidence_only',
        'compile_error': null,
        'compiler_diagnostics': <String, Object?>{
          'capture': 'captured',
          'messages': <Object?>[],
          'omitted': 0,
        },
        'install_restore': 'restored_exact',
        'recovery_required': false,
        'output_discarded': true,
      },
      'scope': 'compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedEntityId: revision3NpcInspectionNpcId,
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
  );
  return ManagedRevision3CompilerCheckReceipt(
    result: result,
    storeStillExactCurrent: true,
  );
}

AuthoringRevision3QuestSourceInspectionResult _questSourceInspection({
  required Revision3QuestOutlineFixture fixture,
  required AuthoringWorkingHead expectedHead,
}) {
  final projectJson = fixture.projectJson;
  final projectBytes = utf8.encode(projectJson);
  final projectSeal = <String, Object?>{
    'byte_len': projectBytes.length,
    'sha256': crypto.sha256.convert(projectBytes).toString(),
  };
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final target = (project['target']! as Map).cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  final questPayload = (quest['payload']! as Map).cast<String, Object?>();
  final questData = (questPayload['data']! as Map).cast<String, Object?>();
  final input = (questData['input']! as Map).cast<String, Object?>();
  final inputBytes = utf8.encode(jsonEncode(input));
  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  final modulePayload = (module['payload']! as Map).cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  final source = moduleData['source']! as String;
  final sourceBytes = utf8.encode(source);
  final sourceSha256 = moduleData['source_sha256']! as String;
  final questRef = <String, Object?>{
    'project_id': revision3QuestOutlineProjectId,
    'id': revision3QuestOutlineQuestId,
    'expected_kind': 'quest_draft',
  };
  final plan = <String, Object?>{
    'format': 'revision3_quest_source_inspection_plan',
    'schema_revision': 3,
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': revision3QuestOutlineProjectId,
      'project_revision': fixture.projectRevision,
      'target_executable': target['executable'],
      'canonical_project': projectSeal,
      'collision_basis_head': jsonDecode(expectedHead.canonicalJson),
      'collision_basis_project': projectSeal,
      'collision_nonquest_project': _homeSealJson(projectBytes.length, 'd'),
      'collision_prior_quest_count': 0,
      'collision_prior_quest_evidence': _homeSealJson(64, '1'),
      'collision_artifact': _homeSealJson(123, 'e'),
      'collision_source': _homeSealJson(123, 'f'),
    },
    'module': <String, Object?>{
      'quest': questRef,
      'script_module': <String, Object?>{
        'project_id': revision3QuestOutlineProjectId,
        'id': revision3QuestOutlineModuleId,
        'expected_kind': 'script_module',
      },
      'draft_input': <String, Object?>{
        'byte_len': inputBytes.length,
        'sha256': crypto.sha256.convert(inputBytes).toString(),
      },
      'persisted_source': <String, Object?>{
        'byte_len': sourceBytes.length,
        'sha256': sourceSha256,
      },
      'generated': <String, Object?>{
        'generator_id': moduleData['generator_id'],
        'generator_version': moduleData['generator_version'],
        'owner': questRef,
        'module_namespace': moduleData['module_namespace'],
        'module_relative_path': moduleData['module_relative_path'],
        'source': source,
        'source_sha256': sourceSha256,
        'input_fingerprint': moduleData['input_fingerprint'],
        'status': moduleData['status'],
      },
    },
  };
  final planJson = jsonEncode(plan);
  final planBytes = utf8.encode(planJson);
  return AuthoringRevision3QuestSourceInspectionResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'inspection_only',
      'head_json': expectedHead.canonicalJson,
      'project_id': revision3QuestOutlineProjectId,
      'project_revision': fixture.projectRevision,
      'project_seal': projectSeal,
      'quest_id': revision3QuestOutlineQuestId,
      'plan_json': planJson,
      'plan_seal': <String, Object?>{
        'byte_len': planBytes.length,
        'sha256': crypto.sha256.convert(planBytes).toString(),
      },
      'scope': 'source_inspection_only',
      'build_status': 'blocked',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: expectedHead,
    requestedQuestId: revision3QuestOutlineQuestId,
  );
}

ManagedRevision3CompilerCheckReceipt _questManagedCompilerReceipt({
  required Revision3QuestOutlineFixture fixture,
  required AuthoringWorkingHead expectedHead,
}) {
  final project = (jsonDecode(fixture.projectJson) as Map)
      .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  final payload = (module['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  final result = AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'compiler_check_only',
      'exact_current': true,
      'head_json': expectedHead.canonicalJson,
      'project': <String, Object?>{
        'id': revision3QuestOutlineProjectId,
        'revision': fixture.projectRevision,
        'seal': <String, Object?>{
          'byte_len': expectedHead.snapshotByteLength,
          'sha256': expectedHead.snapshotSha256,
        },
      },
      'entity': <String, Object?>{
        'kind': 'quest_draft',
        'id': revision3QuestOutlineQuestId,
        'revision': fixture.questRevision,
      },
      'module': <String, Object?>{
        'id': revision3QuestOutlineModuleId,
        'revision': fixture.moduleRevision,
        'namespace': data['module_namespace'],
        'relative_path': data['module_relative_path'],
        'source_sha256': data['source_sha256'],
      },
      'compiler': <String, Object?>{
        'outcome': 'compiled_evidence_only',
        'compile_error': null,
        'compiler_diagnostics': <String, Object?>{
          'capture': 'captured',
          'messages': <Object?>[],
          'omitted': 0,
        },
        'install_restore': 'restored_exact',
        'recovery_required': false,
        'output_discarded': true,
      },
      'scope': 'compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: expectedHead,
    requestedEntityId: revision3QuestOutlineQuestId,
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.questDraft,
  );
  return ManagedRevision3CompilerCheckReceipt(
    result: result,
    storeStillExactCurrent: true,
  );
}

Map<String, Object?> _homeSealJson(int byteLength, String digit) =>
    <String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, digit).join(),
    };

Revision3ContentIndex _npcInspectionIndex({
  required String projectId,
  required int revision,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Home NPC project',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256':
          'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{'npc_draft': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': revision3NpcInspectionNpcId,
      'kind': 'npc_draft',
      'display_name': 'Inspection Guard',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': revision3NpcInspectionUniqueName,
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': revision3NpcInspectionUniqueName,
          'module_namespace': revision3NpcInspectionModuleNamespace,
          'parent_character_definition':
              'UCharacterDefinition_Human_OM_GRD_Asghan_263',
          'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
          'parent_spawn_definition':
              'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

Revision3ContentIndex _storyWorkbenchGameGateIndex({
  required String projectId,
  required int revision,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Home Story Workbench project',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256':
          'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{
    'npc_draft': 1,
    'quest_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    <String, Object?>{
      'id': revision3NpcInspectionNpcId,
      'kind': 'npc_draft',
      'display_name': 'Inspection Guard',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': revision3NpcInspectionUniqueName,
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': revision3NpcInspectionUniqueName,
          'module_namespace': revision3NpcInspectionModuleNamespace,
          'parent_character_definition':
              'UCharacterDefinition_Human_OM_GRD_Asghan_263',
          'parent_ai_agent_config': 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
          'parent_spawn_definition':
              'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': revision3QuestOutlineQuestId,
      'kind': 'quest_draft',
      'display_name': 'Find Homer',
      'revision': 4,
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
            'project_id': projectId,
            'entity_id': revision3QuestOutlineModuleId,
            'expected_kind': 'script_module',
          },
          'resolution': 'resolved',
        },
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': revision3QuestOutlineModuleId,
      'kind': 'script_module',
      'display_name': 'Find Homer Script',
      'revision': 5,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': projectId,
          'entity_id': revision3QuestOutlineQuestId,
          'expected_kind': 'quest_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
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
            'entity_id': revision3QuestOutlineQuestId,
            'expected_kind': 'quest_draft',
          },
          'resolution': 'resolved',
        },
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

Revision3ContentIndex _questTranscriptHomeIndex({required int revision}) {
  final json = revision3VoiceContentIndexJsonFixture(revision: revision);
  final counts = (json['entity_counts']! as Map).cast<String, Object?>();
  counts['quest_draft'] = 1;
  counts['script_module'] = 1;
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  final localizationSummary = (localization['summary']! as Map)
      .cast<String, Object?>();
  final localizationData = (localizationSummary['data']! as Map)
      .cast<String, Object?>();
  localizationData['locales'] = <Object?>['de', 'en'];
  entities.addAll(<Map<String, Object?>>[
    <String, Object?>{
      'id': _homeQuestTranscriptQuestId,
      'kind': 'quest_draft',
      'display_name': 'Find Homer',
      'revision': 4,
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
          'transcript_count': 1,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
          'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
        },
      },
      'references': <Object?>[
        _homeContentReference(
          role: 'draft_script_module',
          targetId: _homeQuestTranscriptModuleId,
          expectedKind: 'script_module',
        ),
        _homeContentReference(
          role: 'quest_transcript_line',
          qualifier: '1',
          targetId: revision3VoiceContentLineId,
          expectedKind: 'dialog_line',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _homeQuestTranscriptModuleId,
      'kind': 'script_module',
      'display_name': 'Find Homer Script',
      'revision': 5,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': revision3VoiceContentProjectId,
          'entity_id': _homeQuestTranscriptQuestId,
          'expected_kind': 'quest_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _homeContentReference(
          role: 'origin_owner',
          targetId: _homeQuestTranscriptQuestId,
          expectedKind: 'quest_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ]);
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _npcGreetingHomeIndex({
  required int revision,
  bool existingDeSlot = true,
  int lineRevision = 2,
  int slotRevision = 1,
  String slotId = revision3VoiceContentSlotId,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingDeSlot,
  );
  final counts = (json['entity_counts']! as Map).cast<String, Object?>();
  counts['npc_draft'] = 1;
  counts['script_module'] = 1;
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final line = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLineId,
  );
  line['revision'] = lineRevision;
  if (existingDeSlot) {
    final slot = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentSlotId,
    );
    slot['id'] = slotId;
    slot['revision'] = slotRevision;
    final lineReferences = (line['references']! as List<Object?>)
        .cast<Map<String, Object?>>();
    final slotReference = lineReferences.singleWhere(
      (reference) => reference['role'] == 'dialog_voice_slot',
    );
    final slotTarget = (slotReference['target']! as Map)
        .cast<String, Object?>();
    slotTarget['entity_id'] = slotId;
  }
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  final localizationSummary = (localization['summary']! as Map)
      .cast<String, Object?>();
  final localizationData = (localizationSummary['data']! as Map)
      .cast<String, Object?>();
  localizationData['locales'] = <Object?>['de', 'en'];
  entities.addAll(<Map<String, Object?>>[
    <String, Object?>{
      'id': _homeNpcGreetingNpcId,
      'kind': 'npc_draft',
      'display_name': 'Asghan',
      'revision': 4,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'OM_GRD_Asghan_263',
      },
      'summary': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'unique_name': 'OM_GRD_Asghan_263',
          'module_namespace': 'PROJECT.NPCS.ASGHAN',
          'parent_character_definition': 'C_HUMAN',
          'parent_ai_agent_config': 'AIV_HUMAN',
          'parent_spawn_definition': 'SPAWN_HUMAN',
          'greeting_count': 1,
        },
      },
      'references': <Object?>[
        _homeContentReference(
          role: 'draft_script_module',
          targetId: _homeNpcGreetingModuleId,
          expectedKind: 'script_module',
        ),
        _homeContentReference(
          role: 'npc_greeting_line',
          targetId: revision3VoiceContentLineId,
          expectedKind: 'dialog_line',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _homeNpcGreetingModuleId,
      'kind': 'script_module',
      'display_name': 'Asghan Script',
      'revision': 5,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-npc-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': revision3VoiceContentProjectId,
          'entity_id': _homeNpcGreetingNpcId,
          'expected_kind': 'npc_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-npc-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.NPCS.ASGHAN',
          'module_relative_path': 'PROJECT/NPCS/ASGHAN.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _homeContentReference(
          role: 'origin_owner',
          targetId: _homeNpcGreetingNpcId,
          expectedKind: 'npc_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ]);
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  return Revision3ContentIndex.fromJsonObject(json);
}

Map<String, Object?> _homeContentReference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Revision3ContentIndex _voiceSelectionIndex({
  required int revision,
  required String selectedTakeId,
  int slotRevision = 1,
  String alternateStatus = 'approved',
  int alternateRevision = 0,
  int selectedRevision = 0,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 2,
    existingSlotHasSelectedTake: true,
  );
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  for (final entity in entities) {
    if (entity['id'] == revision3VoiceContentSlotId) {
      entity['revision'] = slotRevision;
      final references = (entity['references']! as List)
          .cast<Map<String, Object?>>();
      final selected = references.singleWhere(
        (reference) => reference['role'] == 'voice_selected',
      );
      final target = (selected['target']! as Map).cast<String, Object?>();
      target['entity_id'] = selectedTakeId;
      selected['target'] = target;
    }
    if (entity['kind'] == 'voice_take') {
      final summary = (entity['summary']! as Map).cast<String, Object?>();
      final data = (summary['data']! as Map).cast<String, Object?>();
      final selected = entity['id'] == selectedTakeId;
      data['status'] = selected ? 'approved' : alternateStatus;
      summary['data'] = data;
      entity['summary'] = summary;
      entity['revision'] = selected ? selectedRevision : alternateRevision;
    }
  }
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _voiceIndexWithUnresolvedLocalization({
  required int revision,
}) {
  final json = revision3VoiceContentIndexJsonFixture(revision: revision);
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  final line = entities.singleWhere(
    (entity) => entity['kind'] == 'dialog_line',
  );
  final references = (line['references']! as List).cast<Map<String, Object?>>();
  final localization = references.singleWhere(
    (reference) => reference['role'] == 'dialog_localization',
  );
  final target = (localization['target']! as Map).cast<String, Object?>();
  target['entity_id'] = '99999999999999999999999999999999';
  localization['resolution'] = 'missing_entity';
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3QuestCatalog _questCatalog() => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: 'parent-one',
      displayName: 'Chapter One',
      runtimeClass: 'UQuest_ChapterOne',
    ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: 'giver-asghan',
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
    ),
  ],
);

Revision3QuestCatalog _questContextCatalog(
  Revision3QuestOutlineFixture fixture,
) => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: 'context-parent-current',
      displayName: 'Chapter Two',
      runtimeClass: 'UQuest_SwampCamp_SCChapter2',
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter2',
      sourceSeal: _homeSeal(11, '1'),
    ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: 'context-giver-current',
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: 'OM_GRD_Asghan_263',
      sourceSeal: _homeSeal(12, '2'),
    ),
  ],
  catalogSeal: fixture.storyCatalogSeal,
  generationExecutableSeal: _homeSeal(171698176, 'a'),
);

AuthoringDraftContentSeal _homeSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': bytes,
      'sha256': List<String>.filled(64, digit).join(),
    });

Revision3NpcCatalog _npcCatalog() => Revision3NpcCatalog(
  choices: [
    Revision3NpcCatalogChoice(
      catalogId: 'g1r:npc:om_grd_asghan_263',
      displayName: 'Asghan guard',
    ),
  ],
);

Revision3BaseGameContentCatalog _baseGameCatalog() =>
    Revision3BaseGameContentCatalog(
      npcs: _npcCatalog(),
      quests: _questCatalog(),
    );

AuthoringRevision3VoiceBuildPlanResult _readyVoicePlan(
  _FakeManagedLease lease,
) => AuthoringRevision3VoiceBuildPlanResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'ready',
    'basis_head_json': lease.head.canonicalJson,
    'project_id': lease.projectId,
    'project_revision': lease.projectRevision,
    'total_slots': 1,
    'ready_slots': 1,
    'blockers': const <Object?>[],
    'plan_authority': 'read_only_voice_build_plan_v1',
    'build_authority': 'not_granted',
    'deployment_status': 'not_performed',
  },
  expectedHead: lease.head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    projectId: lease.projectId,
    projectRevision: lease.projectRevision,
  ),
);

AuthoringRevision3VoiceBuildPlanResult _unresolvedVoicePlan(
  _FakeManagedLease lease,
) {
  final readyJson = revision3VoiceFixtureBuildReadyProjectJson(
    projectId: lease.projectId,
    projectRevision: lease.projectRevision,
  );
  final readyProject = (jsonDecode(readyJson) as Map).cast<String, Object?>();
  final readyEntities = (readyProject['entities']! as Map)
      .cast<String, Object?>();
  final originalSlotId = readyEntities.entries.singleWhere((entry) {
    final entity = (entry.value! as Map).cast<String, Object?>();
    final payload = (entity['payload']! as Map).cast<String, Object?>();
    return payload['kind'] == 'voice_slot';
  }).key;
  final contentBoundJson = readyJson
      .replaceAll(
        revision3VoiceFixtureLocalizationId,
        revision3VoiceContentLocalizationId,
      )
      .replaceAll(revision3VoiceFixtureLineId, revision3VoiceContentLineId)
      .replaceAll(originalSlotId, revision3VoiceContentSlotId);
  final project = (jsonDecode(contentBoundJson) as Map).cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final slot = (entities[revision3VoiceContentSlotId]! as Map)
      .cast<String, Object?>();
  final payload = (slot['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['target_resolution'] = <String, Object?>{'state': 'unresolved'};
  final projectJson = jsonEncode(project);
  return AuthoringRevision3VoiceBuildPlanResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'blocked',
      'basis_head_json': lease.head.canonicalJson,
      'project_id': lease.projectId,
      'project_revision': lease.projectRevision,
      'total_slots': 1,
      'ready_slots': 0,
      'blockers': <Object?>[
        <String, Object?>{
          'slot_id': revision3VoiceContentSlotId,
          'line_id': revision3VoiceContentLineId,
          'line_label': 'Asghan greeting',
          'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
          'locale': 'de',
          'reason': 'unresolved_target',
        },
      ],
      'plan_authority': 'read_only_voice_build_plan_v1',
      'build_authority': 'not_granted',
      'deployment_status': 'not_performed',
    },
    expectedHead: lease.head,
    expectedProjectJson: projectJson,
  );
}

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
