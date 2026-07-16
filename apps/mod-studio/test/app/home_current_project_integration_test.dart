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
import 'package:gore_mod/project/revision3_base_game_content_browser.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_global_content_search.dart';
import 'package:gore_mod/project/revision3_managed_compiler_check_panel.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_wizard.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_build_dialog.dart';
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
import '../support/revision3_quest_outline_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

void main() {
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
      expect(find.byType(NavigationRail), findsOneWidget);
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
    'managed project copy is available without a game from menu and first tool card, with one global busy lane',
    (tester) async {
      await _setDesktopTestSurface(tester);
      final parent = Directory.systemTemp.createTempSync('gore_home_export_');
      addTearDown(() => parent.deleteSync(recursive: true));
      final completion =
          Completer<AuthoringRevision3ExactSnapshotExportResult>();
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
      final card = find.byKey(const Key('managed-export-project-copy'));
      expect(card, findsOneWidget);
      expect(tester.widget<InkWell>(card).onTap, isNotNull);

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

      await tester.ensureVisible(card);
      await tester.pumpAndSettle();
      await tester.tap(card);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-export-dialog')),
        findsOneWidget,
      );
      expect(exportItem().enabled, isFalse);
      expect(
        tester
            .widget<InkWell>(
              find.byKey(
                const Key('managed-export-project-copy'),
                skipOffstage: false,
              ),
            )
            .onTap,
        isNull,
      );

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
    'dirty managed project text guards Open and Close without losing the draft',
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
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      await coordinator.openManagedRevision3(managed.root);
      final container = _container(
        coordinator: coordinator,
        pickManaged: (_) async {
          pickerCalls++;
          return r'C:\mods\replacement-managed-project';
        },
      );
      addTearDown(container.dispose);

      await _pumpApp(tester, container);
      await tester.pumpAndSettle();
      await _navigateManagedLocalizationVoice(tester);
      expect(managed.dialogLocalizationEditSeedCalls, 1);

      final textField = find.byKey(const Key('revision3-localization-text-de'));
      expect(textField, findsOneWidget);
      final l10n = AppLocalizations.of(tester.element(textField));
      await tester.enterText(textField, draft);
      await tester.pump();
      await tester.pump();

      final projectMenu = find.byKey(const Key('project-menu'));
      final exportItem = tester
          .widget<PopupMenuButton<String>>(projectMenu)
          .itemBuilder(tester.element(projectMenu))
          .whereType<PopupMenuItem<String>>()
          .singleWhere(
            (item) => item.key == const Key('project-export-managed-revision3'),
          );
      expect(exportItem.enabled, isFalse);
      expect(
        tester
            .widget<InkWell>(
              find.byKey(
                const Key('managed-export-project-copy'),
                skipOffstage: false,
              ),
            )
            .onTap,
        isNull,
      );
      expect(
        find.text(l10n.projectExportActionDirtyBlocked, skipOffstage: false),
        findsOneWidget,
      );

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
      expect(find.byKey(const Key('managed-review-problems')), findsOneWidget);

      await _navigateManagedDataAssets(tester);
      expect(managed.dataAssetListCalls, 1);
      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-nav-story'),
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
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-story-workspace-create-quest')),
            )
            .onPressed,
        isNull,
      );
      expect(
        find.byKey(const Key('revision3-story-workspace-empty')),
        findsOneWidget,
      );

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-nav-content'),
      );
      expect(
        find.byKey(const Key('revision3-content-workspace-page-data-assets')),
        findsOneWidget,
        reason: 'Content remembers its last secondary route',
      );
      expect(managed.dataAssetListCalls, 1);

      await _navigateManagedWorkspace(
        tester,
        const Key('revision3-project-workspace-nav-story'),
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
        const Key('revision3-project-workspace-nav-world'),
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
        const Key('revision3-project-workspace-nav-localization-voice'),
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
        const Key('revision3-project-workspace-nav-validate-test'),
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
        const Key('revision3-project-workspace-nav-build-release'),
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
        const Key('revision3-project-workspace-nav-history'),
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
        const Key('revision3-project-workspace-nav-settings-expert'),
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
        const Key('revision3-project-workspace-nav-story'),
      );

      final createNpc = find.byKey(
        const Key('revision3-story-workspace-create-npc'),
      );
      expect(tester.widget<FilledButton>(createNpc).onPressed, isNotNull);
      await tester.tap(createNpc);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
      await tester.tap(find.byKey(const Key('revision3-npc-cancel')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);

      final createQuest = find.byKey(
        const Key('revision3-story-workspace-create-quest'),
      );
      expect(tester.widget<OutlinedButton>(createQuest).onPressed, isNotNull);
      await tester.tap(createQuest);
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
        const Key('revision3-project-workspace-nav-story'),
      );
      expect(
        find.byKey(const Key('revision3-story-workspace-empty')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-create-quest')),
      );
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
        const Key('revision3-project-workspace-nav-story'),
      );
      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-create-quest')),
      );
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
      final switchedRail = find.byKey(
        const Key('revision3-project-workspace-rail'),
        skipOffstage: false,
      );
      expect(switchedRail, findsOneWidget);
      tester.widget<NavigationRail>(switchedRail).onDestinationSelected!(2);
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
      const projectId = '63636363636363636363636363636363';
      final managed = _FakeManagedLease(
        root: Directory(r'C:\mods\story-compact-german'),
        projectId: projectId,
        projectRevision: 7,
        head: _head(7),
        contentIndexBuilder: (lease) => _storyWorkbenchGameGateIndex(
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
      expect(tester.takeException(), isNull);
      expect(
        Localizations.localeOf(
          tester.element(find.byKey(const Key('revision3-project-workspace'))),
        ).languageCode,
        'de',
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-workspace-narrow-menu')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-project-workspace-nav-story')),
      );
      await tester.pumpAndSettle();

      final createNpc = find.byKey(
        const Key('revision3-story-workspace-create-npc'),
      );
      final createQuest = find.byKey(
        const Key('revision3-story-workspace-create-quest'),
      );
      expect(tester.widget<FilledButton>(createNpc).onPressed, isNull);
      expect(tester.widget<OutlinedButton>(createQuest).onPressed, isNull);
      final missingGame = AppLocalizations.of(
        tester.element(createNpc),
      ).managedDashboardMissingGameDescription;
      expect(
        missingGame,
        'Richte die Gothic-1-Remake-Installation in den Einstellungen ein, '
        'bevor du Aktionen verwendest, die Nachweise aus dem installierten '
        'Spiel benötigen.',
      );
      expect(find.text(missingGame), findsOneWidget);
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

      final edit = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-overview-'
          '$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, edit);
      expect(_workbenchActionTileWidget(tester, edit).enabled, isTrue);
      await _tapWorkbenchAction(tester, edit);
      await tester.pumpAndSettle();
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

  testWidgets('Problems opens the exact referenced entity in Content checks', (
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
      const Key('revision3-project-workspace-nav-validate-test'),
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
        const Key('revision3-project-workspace-nav-validate-test'),
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
        tester
            .widget<DropdownButtonFormField<String>>(
              find.byType(DropdownButtonFormField<String>),
            )
            .initialValue,
        'de',
      );
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
        const Key('revision3-project-workspace-nav-validate-test'),
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

      final createButton = find.byKey(const Key('managed-create-quest-draft'));
      expect(createButton, findsOneWidget);
      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      expect(tester.widget<InkWell>(createButton).onTap, isNotNull);
      expect(managed.contentReadCalls, 1);

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-create-quest-draft'),
      );
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
      await _expandManagedTechnicalDetails(tester);
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
      final edit = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-overview-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, edit);
      await _tapWorkbenchAction(tester, edit);
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
      expect(managed.contentReadCalls, 4);
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
          const Key('revision3-content-entity-$revision3QuestOutlineQuestId'),
        ),
      );
      await tester.pumpAndSettle();

      final storyTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-story-$revision3QuestOutlineQuestId',
        ),
      );
      await tester.ensureVisible(storyTab);
      await tester.tap(storyTab);
      await tester.pumpAndSettle();
      final editStory = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-story-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, editStory);
      expect(_workbenchActionTileWidget(tester, editStory).enabled, isFalse);
      expect(
        find.descendant(
          of: editStory,
          matching: find.text(missingGameReason, skipOffstage: false),
          skipOffstage: false,
        ),
        findsOneWidget,
      );

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
      final storyTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-story-$revision3QuestOutlineQuestId',
        ),
      );
      expect(storyTab, findsOneWidget);
      await tester.ensureVisible(storyTab);
      await tester.tap(storyTab);
      await tester.pumpAndSettle();
      final editStory = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-story-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, editStory);
      expect(_workbenchActionTileWidget(tester, editStory).enabled, isTrue);
      await _tapWorkbenchAction(tester, editStory);
      await tester.pumpAndSettle();
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
      expect(managed.contentReadCalls, 4);
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
      final logicTab = find.byKey(
        const Key(
          'revision3-story-workbench-tab-logic-$revision3QuestOutlineQuestId',
        ),
      );
      expect(logicTab, findsOneWidget);
      await tester.ensureVisible(logicTab);
      await tester.tap(logicTab);
      await tester.pumpAndSettle();
      final editLogic = _storyWorkbenchAction(
        const Key(
          'revision3-story-workbench-action-edit-logic-$revision3QuestOutlineQuestId',
        ),
      );
      await _revealWorkbenchAction(tester, editLogic);
      expect(_workbenchActionTileWidget(tester, editLogic).enabled, isTrue);
      await _tapWorkbenchAction(tester, editLogic);
      await tester.pumpAndSettle();
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

      expect(managed.questTransitionsSeedCalls, 1);
      expect(managed.questTransitionsPublishCalls, 1);
      expect(managed.contentReadCalls, 4);
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
      expect(tester.widget<InkWell>(createButton).onTap, isNotNull);
      expect(managed.contentReadCalls, 1);

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-create-npc-draft'),
      );
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
      await _expandManagedTechnicalDetails(tester);
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

      final createButton = find.byKey(const Key('managed-create-dialog-line'));
      expect(createButton, findsOneWidget);
      expect(tester.widget<InkWell>(createButton).onTap, isNotNull);
      final l10n = AppLocalizations.of(tester.element(createButton));

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-create-dialog-line'),
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
        tester
            .widget<TextFormField>(
              find.byKey(const Key('revision3-voice-line-search')),
            )
            .controller
            ?.text,
        'Asghan — Mine entrance warning',
      );
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key('revision3-voice-locale')),
            )
            .controller
            ?.text,
        'de',
      );
      expect(managed.voicePublishCalls, 0);

      await tester.tap(find.byKey(const Key('revision3-voice-cancel')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<InkWell>(find.byKey(const Key('managed-add-voice-take')))
            .onTap,
        isNotNull,
      );
      expect(
        find.text(l10n.managedActionNewDialogLineSaved(8)),
        findsOneWidget,
      );
      expect(find.text('Build / Deploy'), findsNothing);
    },
  );

  testWidgets(
    'Home reuses exact project text only after bounded preview verification',
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

      final createButton = find.byKey(const Key('managed-create-dialog-line'));
      final l10n = AppLocalizations.of(tester.element(createButton));
      await _tapManagedDashboardAction(
        tester,
        const Key('managed-create-dialog-line'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.managedDialogLineReuseMode));
      await tester.pump();

      expect(find.text(displayName), findsOneWidget);
      expectTechnicalIdentityHidden();
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        'Reused mine warning',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-speaker')),
        'Asghan',
      );
      await tester.tap(find.text(displayName));
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

      final voiceButton = find.byKey(const Key('managed-add-voice-take'));
      expect(voiceButton, findsOneWidget);
      expect(tester.widget<InkWell>(voiceButton).onTap, isNotNull);
      expect(
        tester
            .widget<InkWell>(find.byKey(const Key('managed-create-npc-draft')))
            .onTap,
        isNotNull,
      );

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-add-voice-take'),
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

      final dashboardAction = find.byKey(const Key('managed-add-voice-take'));
      expect(dashboardAction, findsOneWidget);
      expect(tester.widget<InkWell>(dashboardAction).onTap, isNull);
      final dashboardPrerequisite = AppLocalizations.of(
        tester.element(dashboardAction),
      ).managedActionAddVoiceTakeRequiresDialogLine;
      expect(
        find.descendant(
          of: dashboardAction,
          matching: find.text(dashboardPrerequisite),
        ),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      await _navigateManagedLocalizationVoice(tester);

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
      expect(tester.takeException(), isNull);
      expect(managed.voicePublishCalls, 0);
    },
  );

  testWidgets('exact DialogLine enables Voice add on dashboard and section', (
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
          revision3VoiceContentIndexFixture(revision: lease.projectRevision),
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

    final dashboardAction = find.byKey(const Key('managed-add-voice-take'));
    expect(dashboardAction, findsOneWidget);
    expect(tester.widget<InkWell>(dashboardAction).onTap, isNotNull);

    await _navigateManagedLocalizationVoice(tester);

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
    expect(tester.takeException(), isNull);
  });

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

      for (final key in const <Key>[
        Key('managed-add-voice-take'),
        Key('managed-manage-voice-takes'),
        Key('managed-resolve-voice-target'),
      ]) {
        final action = find.byKey(key);
        expect(action, findsOneWidget);
        expect(tester.widget<InkWell>(action).onTap, isNull);
      }

      await _navigateManagedLocalizationVoice(tester);
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
      final dashboardAction = find.byKey(const Key('managed-add-voice-take'));
      expect(dashboardAction, findsOneWidget);
      expect(tester.widget<InkWell>(dashboardAction).onTap, isNull);
      expect(tester.takeException(), isNull);
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

      expect(
        tester
            .widget<InkWell>(
              find.byKey(const Key('managed-manage-voice-takes')),
            )
            .onTap,
        isNotNull,
      );
      expect(
        tester
            .widget<InkWell>(find.byKey(const Key('managed-add-voice-take')))
            .onTap,
        isNull,
        reason: 'Ogg import still needs its separate safety game root',
      );
      final missingGame = AppLocalizations.of(
        tester.element(find.byKey(const Key('managed-add-voice-take'))),
      ).managedDashboardMissingGameDescription;
      for (final key in const <Key>[
        Key('managed-add-voice-take'),
        Key('managed-resolve-voice-target'),
        Key('managed-build-voice-bundle'),
      ]) {
        final action = find.byKey(key);
        expect(tester.widget<InkWell>(action).onTap, isNull);
        expect(
          find.descendant(of: action, matching: find.text(missingGame)),
          findsOneWidget,
        );
      }
      await _navigateManagedContent(tester);
      final libraryLine = find.byKey(
        const Key('revision3-content-entity-$revision3VoiceContentLineId'),
      );
      await tester.ensureVisible(libraryLine);
      await tester.tap(libraryLine);
      await tester.pump();
      expect(tester.widget<ListTile>(libraryLine).selected, isTrue);

      await _navigateManagedHome(tester);
      await _tapManagedDashboardAction(
        tester,
        const Key('managed-manage-voice-takes'),
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
      await _tapManagedDashboardAction(
        tester,
        const Key('managed-manage-voice-takes'),
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

      final resolveTarget = find.byKey(
        const Key('managed-resolve-voice-target'),
      );
      final buildBundle = find.byKey(const Key('managed-build-voice-bundle'));
      expect(tester.widget<InkWell>(resolveTarget).onTap, isNotNull);
      expect(tester.widget<InkWell>(buildBundle).onTap, isNotNull);

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-resolve-voice-target'),
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

      await _tapManagedDashboardAction(
        tester,
        const Key('managed-build-voice-bundle'),
      );
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
    expect(tester.widget<InkWell>(voiceButton).onTap, isNull);
    expect(
      tester
          .widget<InkWell>(find.byKey(const Key('managed-manage-voice-takes')))
          .onTap,
      isNotNull,
      reason: 'selection is project-only and intentionally needs no game path',
    );
    expect(
      tester
          .widget<InkWell>(
            find.byKey(const Key('managed-resolve-voice-target')),
          )
          .onTap,
      isNull,
    );
    expect(
      tester
          .widget<InkWell>(find.byKey(const Key('managed-build-voice-bundle')))
          .onTap,
      isNull,
    );
    expect(
      find.byKey(const Key('revision3-project-dashboard-missing-game')),
      findsOneWidget,
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
      expect(tester.widget<OutlinedButton>(browse).onPressed, isNotNull);
      expect(managed.dataAssetPackageIndexReadCalls, 0);
      await tester.tap(browse);
      await tester.pumpAndSettle();

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
      await _navigateManagedDataAssets(tester);

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
      expect(find.byKey(const Key('managed-create-quest-draft')), findsNothing);
      expect(find.byKey(const Key('managed-manage-voice-takes')), findsNothing);

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
  final details = find.byKey(const Key('managed-project-technical-details'));
  expect(details, findsOneWidget);
  await tester.tap(details);
  await tester.pumpAndSettle();
}

const _managedPrimaryNavigationKeys = <Key>[
  Key('revision3-project-workspace-nav-home'),
  Key('revision3-project-workspace-nav-content'),
  Key('revision3-project-workspace-nav-story'),
  Key('revision3-project-workspace-nav-world'),
  Key('revision3-project-workspace-nav-localization-voice'),
  Key('revision3-project-workspace-nav-validate-test'),
  Key('revision3-project-workspace-nav-build-release'),
  Key('revision3-project-workspace-nav-history'),
  Key('revision3-project-workspace-nav-settings-expert'),
];

Future<void> _navigateManagedHome(WidgetTester tester) =>
    _navigateManagedWorkspace(
      tester,
      const Key('revision3-project-workspace-nav-home'),
    );

Future<void> _navigateManagedContent(WidgetTester tester) async {
  await _navigateManagedWorkspace(
    tester,
    const Key('revision3-project-workspace-nav-content'),
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
    const Key('revision3-project-workspace-nav-content'),
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
    const Key('revision3-project-workspace-nav-content'),
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
    const Key('revision3-project-workspace-nav-content'),
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

Future<void> _navigateManagedWorkspace(WidgetTester tester, Key key) async {
  final destination = find.byKey(key);
  expect(destination, findsOneWidget);
  await tester.tap(destination);
  await tester.pumpAndSettle();
}

Future<void> _navigateManagedLocalizationVoice(
  WidgetTester tester, {
  bool settle = true,
}) async {
  const destinationKey = Key(
    'revision3-project-workspace-nav-localization-voice',
  );
  final narrowMenu = find.byKey(
    const Key('revision3-project-workspace-narrow-menu'),
  );
  if (narrowMenu.evaluate().isNotEmpty) {
    await tester.tap(narrowMenu);
    await tester.pumpAndSettle();
  }
  final destination = find.byKey(destinationKey);
  expect(destination, findsOneWidget);
  await tester.ensureVisible(destination);
  await tester.tap(destination);
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

Future<void> _openStoryWorkbenchEntity(
  WidgetTester tester,
  String entityId,
) async {
  final entity = find.byKey(Key('revision3-content-entity-$entityId'));
  expect(entity, findsOneWidget);
  await tester.tap(entity);
  await tester.pumpAndSettle();
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

Future<void> _tapManagedDashboardAction(WidgetTester tester, Key key) async {
  final action = find.byKey(key);
  expect(action, findsOneWidget);
  await tester.ensureVisible(action);
  await tester.pumpAndSettle();
  await tester.tap(action);
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
typedef _RecoveryCallback =
    FutureOr<ManagedRevision3RecoveryCheckpoint> Function(
      _FakeRecoverableManagedLease lease,
    );
typedef _ProjectExportCallback =
    FutureOr<AuthoringRevision3ExactSnapshotExportResult> Function(
      _FakeExportManagedLease lease,
      String output,
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
    this.onContentIndexRead,
    this.contentIndexBuilder,
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
  int closeCalls = 0;

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  Future<void> close() async => closeCalls++;

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
  }) => throw StateError(
    'fake managed lease has no installed DataAsset inspector',
  );

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
  }) => throw StateError(
    'fake managed lease has no reviewed installed DataAsset edit publisher',
  );

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

final class _FakeHistoryManagedLease extends _FakeManagedLease
    implements ManagedRevision3ProjectHistoryLease {
  _FakeHistoryManagedLease({
    required super.root,
    required super.projectId,
    required super.projectRevision,
    required super.head,
    required this.history,
    super.contentIndexBuilder,
  });

  final Revision3ProjectHistorySnapshot history;
  int historyReadCalls = 0;

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
  }) => throw StateError('history restore is not configured for this fake');

  @override
  void markRequiresReopenAfterHistoryUncertainty() {
    requiresReopenValue = true;
  }
}

final class _FakeExportManagedLease extends _FakeManagedLease
    implements ManagedRevision3ProjectExportLease {
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
  bool get supportsExactSnapshotExport => true;

  @override
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
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
}) {
  final packageIndexJson = jsonEncode(<String, Object?>{
    'status': 'complete_index',
    'physical_chunk_count': 1,
    'winning_export_bundle_count': 1,
    'directory_indexed_export_bundle_count': 1,
    'out_of_scope_export_bundle_count': 0,
    'candidates': <Object?>[
      <String, Object?>{
        'target_path': '/Game/Characters/DA_Asghan',
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

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

AuthoringRevision3ExactSnapshotExportResult _homeProjectExportResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String output,
}) => AuthoringRevision3ExactSnapshotExportResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'exported',
    'format': 'managed_revision3_exact_snapshot_v1',
    'artifact_kind': 'portable_snapshot_review_copy',
    'restore_status': 'not_supported',
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
          'text': 'Bleib stehen!',
          'voice_slot_present': false,
          'candidate_count': 0,
        },
        <String, Object?>{
          'locale': 'en',
          'text': 'Stop right there!',
          'voice_slot_present': false,
          'candidate_count': 0,
        },
      ],
      'line_backlinks': <Object?>[],
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
  'entity_counts': <String, Object?>{'npc_draft': 1, 'quest_draft': 1},
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
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'parent_runtime_class': 'B_Quest_FindHomer_C',
          'giver_runtime_unique_name': 'ASGHAN',
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

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
