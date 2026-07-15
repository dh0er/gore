import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_wizard.dart';

const _gameRoot = r'C:\Games\Gothic Remake';
const _projectId = '11111111111111111111111111111111';
const _questId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _parentOne = 'parent-one';
const _parentTwo = 'parent-two';
const _giverAsghan = 'giver-asghan';
const _giverViper = 'giver-viper';

void main() {
  testWidgets('valid requested choices preselect and publish when not first', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    Revision3QuestDraftAuthoringInput? published;

    await _openWizard(
      tester,
      initialParentCatalogId: _parentTwo,
      initialGiverCatalogId: _giverViper,
      loadCatalog: (_) async {
        loadCalls += 1;
        return _catalog(includeAlternates: true);
      },
      publish: ({required gameRoot, required input}) async {
        published = input;
        return _publication();
      },
    );
    await tester.pumpAndSettle();

    expect(find.text('Chapter Two'), findsOneWidget);
    expect(find.text('Viper'), findsOneWidget);
    expect(find.text(_parentTwo), findsNothing);
    expect(find.text(_giverViper), findsNothing);

    await _fillForm(tester);
    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(published?.parentCatalogId, _parentTwo);
    expect(published?.giverCatalogId, _giverViper);
  });

  testWidgets('unknown requested choices safely use trusted defaults', (
    tester,
  ) async {
    await _setSurface(tester);
    Revision3QuestDraftAuthoringInput? published;
    const unknownParent = 'parent-not-in-exact-catalog';
    const unknownGiver = 'giver-not-in-exact-catalog';

    await _openWizard(
      tester,
      initialParentCatalogId: unknownParent,
      initialGiverCatalogId: unknownGiver,
      loadCatalog: (_) async => _catalog(includeAlternates: true),
      publish: ({required gameRoot, required input}) async {
        published = input;
        return _publication();
      },
    );
    await tester.pumpAndSettle();

    expect(find.text('Chapter One'), findsOneWidget);
    expect(find.text('Asghan'), findsOneWidget);
    expect(find.text(unknownParent), findsNothing);
    expect(find.text(unknownGiver), findsNothing);

    await _fillForm(tester);
    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(published?.parentCatalogId, _parentOne);
    expect(published?.giverCatalogId, _giverAsghan);
    expect(published?.parentCatalogId, isNot(unknownParent));
    expect(published?.giverCatalogId, isNot(unknownGiver));
  });

  testWidgets('fresh catalog reload retains explicit user choices', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    Revision3QuestDraftAuthoringInput? published;

    await _openWizard(
      tester,
      initialParentCatalogId: _parentTwo,
      initialGiverCatalogId: _giverViper,
      loadCatalog: (_) async {
        loadCalls += 1;
        return _catalog(includeAlternates: true);
      },
      publish: ({required gameRoot, required input}) async {
        published = input;
        throw StateError('keep the wizard open');
      },
    );
    await tester.pumpAndSettle();

    await _chooseDropdown(tester, index: 0, label: 'Chapter One');
    await _chooseDropdown(tester, index: 1, label: 'Asghan');
    await _fillForm(tester);
    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(published?.parentCatalogId, _parentOne);
    expect(published?.giverCatalogId, _giverAsghan);
    expect(find.text('Chapter One'), findsOneWidget);
    expect(find.text('Asghan'), findsOneWidget);
    expect(find.text('Chapter Two'), findsNothing);
    expect(find.text('Viper'), findsNothing);
  });

  testWidgets(
    'shows only friendly fields and publishes after a fresh recheck',
    (tester) async {
      await _setSurface(tester);
      var loadCalls = 0;
      String? publishedGameRoot;
      Revision3QuestDraftAuthoringInput? publishedInput;
      Revision3QuestDraftPublication? dialogResult;

      await _openWizard(
        tester,
        loadCatalog: (gameRoot) async {
          expect(gameRoot, _gameRoot);
          loadCalls += 1;
          return _catalog();
        },
        publish: ({required gameRoot, required input}) async {
          publishedGameRoot = gameRoot;
          publishedInput = input;
          return _publication();
        },
        onResult: (result) => dialogResult = result,
      );
      await tester.pumpAndSettle();

      expect(find.text('Offline draft'), findsOneWidget);
      expect(find.text('Build blocked'), findsOneWidget);
      expect(find.text('Runtime unqualified'), findsOneWidget);
      expect(find.text('Chapter One'), findsOneWidget);
      expect(find.text('Asghan'), findsOneWidget);
      expect(find.text('parent-one'), findsNothing);
      expect(find.text('giver-asghan'), findsNothing);
      expect(find.text('UQuest_ChapterOne'), findsNothing);
      expect(find.text('OM_GRD_Asghan_263'), findsNothing);
      expect(find.textContaining('module namespace'), findsNothing);

      await _fillForm(tester);
      await tester.tap(find.byKey(const Key('revision3-quest-submit')));
      await tester.pumpAndSettle();

      expect(loadCalls, 2);
      expect(publishedGameRoot, _gameRoot);
      expect(publishedInput?.title, 'Find Homer');
      expect(publishedInput?.description, 'Homer vanished near the old gate.');
      expect(publishedInput?.objectiveTitle, 'Ask Asghan about Homer');
      expect(publishedInput?.parentCatalogId, 'parent-one');
      expect(publishedInput?.giverCatalogId, 'giver-asghan');
      expect(dialogResult?.projectRevision, 8);
      expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
    },
  );

  testWidgets('fails closed when a selected game choice changed', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async {
        loadCalls += 1;
        return loadCalls == 1
            ? _catalog()
            : _catalog(parentId: 'parent-two', parentName: 'Chapter Two');
      },
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        return _publication();
      },
    );
    await tester.pumpAndSettle();
    await _fillForm(tester);

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(publishCalls, 0);
    expect(find.textContaining('game choices changed'), findsOneWidget);
    final parentPicker = find.byType(DropdownButtonFormField<String>).first;
    await tester.ensureVisible(parentPicker);
    await tester.pumpAndSettle();
    await tester.tap(parentPicker);
    await tester.pumpAndSettle();
    expect(find.text('Chapter Two'), findsOneWidget);
    expect(find.byKey(const Key('revision3-quest-wizard')), findsOneWidget);
  });

  testWidgets('locks authoring when publication requires a reopen', (
    tester,
  ) async {
    await _setSurface(tester);
    await _openWizard(
      tester,
      loadCatalog: (_) async => _catalog(),
      publish: ({required gameRoot, required input}) async =>
          throw const Revision3QuestDraftRequiresReopenException(),
    );
    await tester.pumpAndSettle();
    await _fillForm(tester);

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(find.textContaining('reopen the managed project'), findsOneWidget);
    final submit = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-quest-submit')),
    );
    expect(submit.onPressed, isNull);
    expect(find.text('Close'), findsOneWidget);
  });

  testWidgets('requires a fresh wizard after its project checkpoint changed', (
    tester,
  ) async {
    await _setSurface(tester);
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async => _catalog(),
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        throw const Revision3QuestDraftStaleCheckpointException();
      },
    );
    await tester.pumpAndSettle();
    await _fillForm(tester);

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(
      find.textContaining('changed while this wizard was open'),
      findsOneWidget,
    );
    final submit = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-quest-submit')),
    );
    expect(submit.onPressed, isNull);
    expect(find.text('Close'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pump();
    expect(publishCalls, 1);
  });

  testWidgets('catalog load can retry and a pending load can be dismissed', (
    tester,
  ) async {
    await _setSurface(tester);
    var calls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async {
        calls += 1;
        if (calls == 1) throw StateError('private catalog failure');
        return _catalog();
      },
      publish: ({required gameRoot, required input}) async => _publication(),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('could not be refreshed'), findsOneWidget);
    expect(find.textContaining('private catalog failure'), findsNothing);
    await tester.tap(find.byKey(const Key('revision3-quest-catalog-retry')));
    await tester.pumpAndSettle();
    expect(find.text('Quest name'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-quest-cancel')));
    await tester.pumpAndSettle();

    final pending = Completer<Revision3QuestCatalog>();
    await _openWizard(
      tester,
      loadCatalog: (_) => pending.future,
      publish: ({required gameRoot, required input}) async => _publication(),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-quest-catalog-loading')),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const Key('revision3-quest-cancel')));
    await tester.pumpAndSettle();
    pending.complete(_catalog());
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
  });

  testWidgets('fresh pre-publication check can be cancelled safely', (
    tester,
  ) async {
    await _setSurface(tester);
    final fresh = Completer<Revision3QuestCatalog>();
    var loads = 0;
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) {
        loads += 1;
        return loads == 1 ? Future.value(_catalog()) : fresh.future;
      },
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        return _publication();
      },
    );
    await tester.pumpAndSettle();
    await _fillForm(tester);

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pump();
    expect(
      tester
          .widget<TextButton>(find.byKey(const Key('revision3-quest-cancel')))
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byKey(const Key('revision3-quest-cancel')));
    await tester.pumpAndSettle();
    fresh.complete(_catalog());
    await tester.pump();

    expect(publishCalls, 0);
    expect(tester.takeException(), isNull);
    expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
  });

  testWidgets('authors, reorders, and persists multiple friendly objectives', (
    tester,
  ) async {
    await _setSurface(tester);
    Revision3QuestDraftAuthoringInput? published;
    await _openWizard(
      tester,
      loadCatalog: (_) async => _catalog(),
      publish: ({required gameRoot, required input}) async {
        published = input;
        return _publication();
      },
    );
    await tester.pumpAndSettle();
    await _fillForm(tester);

    final add = find.byKey(const Key('revision3-quest-objective-add'));
    await tester.ensureVisible(add);
    await tester.tap(add);
    await tester.pump();
    await tester.ensureVisible(add);
    await tester.tap(add);
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-quest-objective-1')),
      'Inspect the old gate',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-objective-2')),
      'Report the secured gate',
    );
    final moveThirdUp = find.byKey(const Key('revision3-quest-objective-up-2'));
    await tester.ensureVisible(moveThirdUp);
    await tester.tap(moveThirdUp);
    await tester.pump();

    await tester.tap(find.byKey(const Key('revision3-quest-submit')));
    await tester.pumpAndSettle();

    expect(published?.objectiveTitles, <String>[
      'Ask Asghan about Homer',
      'Report the secured gate',
      'Inspect the old gate',
    ]);
    expect(find.byKey(const Key('revision3-quest-wizard')), findsNothing);
  });
}

Future<void> _setSurface(WidgetTester tester) async {
  await tester.binding.setSurfaceSize(const Size(1100, 900));
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _openWizard(
  WidgetTester tester, {
  required Revision3QuestCatalogLoader loadCatalog,
  required Revision3QuestDraftPublisher publish,
  String? initialParentCatalogId,
  String? initialGiverCatalogId,
  ValueChanged<Revision3QuestDraftPublication?>? onResult,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: FilledButton(
              key: const Key('open-wizard'),
              onPressed: () async {
                final result = await showDialog<Revision3QuestDraftPublication>(
                  context: context,
                  builder: (context) => Revision3QuestWizardDialog(
                    gameRoot: _gameRoot,
                    loadCatalog: loadCatalog,
                    publish: publish,
                    initialParentCatalogId: initialParentCatalogId,
                    initialGiverCatalogId: initialGiverCatalogId,
                  ),
                );
                onResult?.call(result);
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-wizard')));
  await tester.pump();
}

Future<void> _fillForm(WidgetTester tester) async {
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
}

Future<void> _chooseDropdown(
  WidgetTester tester, {
  required int index,
  required String label,
}) async {
  final picker = find.byType(DropdownButtonFormField<String>).at(index);
  await tester.ensureVisible(picker);
  await tester.tap(picker);
  await tester.pumpAndSettle();
  await tester.tap(find.text(label).last);
  await tester.pumpAndSettle();
}

Revision3QuestCatalog _catalog({
  String parentId = _parentOne,
  String parentName = 'Chapter One',
  bool includeAlternates = false,
}) => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: parentId,
      displayName: parentName,
      runtimeClass: parentId == 'parent-one'
          ? 'UQuest_ChapterOne'
          : 'UQuest_ChapterTwo',
    ),
    if (includeAlternates)
      Revision3QuestParentChoice(
        catalogId: _parentTwo,
        displayName: 'Chapter Two',
        runtimeClass: 'UQuest_ChapterTwo',
      ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: _giverAsghan,
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
    ),
    if (includeAlternates)
      Revision3QuestGiverChoice(
        catalogId: _giverViper,
        displayName: 'Viper',
        runtimeUniqueName: 'OC_GRD_Viper_253',
      ),
  ],
);

Revision3QuestDraftPublication _publication() => Revision3QuestDraftPublication(
  projectId: _projectId,
  projectRevision: 8,
  questId: _questId,
  scriptModuleId: _moduleId,
);
