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
import 'package:gore_mod/gore_mod_app.dart';
import 'package:gore_mod/home_page.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:path/path.dart' as p;

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
          .widget<PopupMenuButton<String>>(find.byType(PopupMenuButton<String>))
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
      expect(find.byType(TabBar), findsNothing);
      expect(legacy.closeCalls, 1);

      expect(find.byKey(const Key('managed-open-settings')), findsOneWidget);
      await tester.tap(find.byKey(const Key('managed-open-settings')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('managed-settings-dialog')), findsOneWidget);
      await tester.tap(find.byKey(const Key('managed-settings-close')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('managed-settings-dialog')), findsNothing);

      final menuFinder = find.byType(PopupMenuButton<String>);
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
        .widget<PopupMenuButton<String>>(find.byType(PopupMenuButton<String>))
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
          .widget<PopupMenuButton<String>>(find.byType(PopupMenuButton<String>))
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

      final menuFinder = find.byType(PopupMenuButton<String>);
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
          .widget<PopupMenuButton<String>>(find.byType(PopupMenuButton<String>))
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
    this.onQuestPublish,
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
  final Revision3QuestDraftPublication Function(
    _FakeManagedLease lease,
    String gameRoot,
    Revision3QuestDraftAuthoringInput input,
  )?
  onQuestPublish;
  final Revision3ContentIndex Function(_FakeManagedLease lease)?
  contentIndexBuilder;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int questPublishCalls = 0;
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
