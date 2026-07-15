import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_wizard.dart';

const _gameRoot = r'C:\Games\Gothic Remake';
const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _asghanCatalogId = 'g1r:npc:om_grd_asghan_263';
const _viperCatalogId = 'g1r:npc:oc_grd_viper_253';

void main() {
  testWidgets('valid initial archetype can publish without opening picker', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    var chooserCalls = 0;
    Revision3NpcDraftAuthoringInput? publishedInput;

    await _openWizard(
      tester,
      initialCatalogId: _asghanCatalogId,
      loadCatalog: (_) async {
        loadCalls += 1;
        return _catalog();
      },
      chooseArchetype: (_, _) async {
        chooserCalls += 1;
        return _viperCatalogId;
      },
      publish: ({required gameRoot, required input}) async {
        publishedInput = input;
        return _publication();
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-npc-selected-archetype')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-npc-selected-archetype-label')),
      findsOneWidget,
    );
    expect(find.text('Asghan'), findsOneWidget);
    expect(find.text(_asghanCatalogId), findsNothing);

    await tester.enterText(
      find.byKey(const Key('revision3-npc-display-name')),
      'North Gate Guard',
    );
    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(chooserCalls, 0);
    expect(publishedInput?.parentCatalogId, _asghanCatalogId);
  });

  testWidgets('unknown initial archetype still requires a trusted choice', (
    tester,
  ) async {
    await _setSurface(tester);
    var publishCalls = 0;

    await _openWizard(
      tester,
      initialCatalogId: 'g1r:npc:not-in-the-exact-catalog',
      loadCatalog: (_) async => _catalog(),
      chooseArchetype: (_, _) async => _asghanCatalogId,
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        return _publication();
      },
    );
    await tester.pumpAndSettle();

    expect(find.text('No archetype selected'), findsOneWidget);
    expect(find.text('g1r:npc:not-in-the-exact-catalog'), findsNothing);
    await tester.enterText(
      find.byKey(const Key('revision3-npc-display-name')),
      'North Gate Guard',
    );
    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(find.text('Choose a character archetype.'), findsOneWidget);
    expect(publishCalls, 0);
  });

  testWidgets('fresh catalog recheck retains explicit user selection', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;

    await _openWizard(
      tester,
      initialCatalogId: _asghanCatalogId,
      loadCatalog: (_) async {
        loadCalls += 1;
        return _catalog();
      },
      chooseArchetype: (_, _) async => _viperCatalogId,
      publish: ({required gameRoot, required input}) async {
        throw StateError('keep the wizard open');
      },
    );
    await tester.pumpAndSettle();
    expect(find.text('Asghan'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('revision3-npc-display-name')),
      'North Gate Guard',
    );
    await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
    await tester.pumpAndSettle();
    expect(find.text('Viper'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(find.text('Viper'), findsOneWidget);
    expect(find.text('Asghan'), findsNothing);
  });

  testWidgets('publishes friendly input after a fresh catalog recheck', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    var chooserCalls = 0;
    Revision3NpcDraftAuthoringInput? publishedInput;
    Revision3NpcDraftPublication? dialogResult;

    await _openWizard(
      tester,
      loadCatalog: (gameRoot) async {
        expect(gameRoot, _gameRoot);
        loadCalls += 1;
        return _catalog();
      },
      chooseArchetype: (context, catalog) async {
        chooserCalls += 1;
        expect(catalog.contains(_asghanCatalogId), isTrue);
        return _asghanCatalogId;
      },
      publish: ({required gameRoot, required input}) async {
        expect(gameRoot, _gameRoot);
        publishedInput = input;
        return _publication();
      },
      onResult: (result) => dialogResult = result,
    );
    await tester.pumpAndSettle();

    expect(find.text('Offline draft'), findsOneWidget);
    expect(find.text('Build blocked'), findsOneWidget);
    expect(find.text('Runtime unqualified'), findsOneWidget);
    expect(find.text('Not spawned'), findsOneWidget);
    expect(find.text(_asghanCatalogId), findsNothing);
    expect(find.textContaining('module namespace'), findsNothing);

    await tester.enterText(
      find.byKey(const Key('revision3-npc-display-name')),
      'North Gate Guard',
    );
    await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
    await tester.pumpAndSettle();
    expect(find.text('Asghan'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(chooserCalls, 1);
    expect(publishedInput?.displayName, 'North Gate Guard');
    expect(publishedInput?.parentCatalogId, _asghanCatalogId);
    expect(dialogResult?.projectRevision, 8);
    expect(find.byKey(const Key('revision3-npc-wizard')), findsNothing);
  });

  testWidgets('fails closed when the selected archetype changed', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async {
        loadCalls += 1;
        return loadCalls == 1 ? _catalog() : _catalog(onlyViper: true);
      },
      chooseArchetype: (_, _) async => _asghanCatalogId,
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        return _publication();
      },
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-npc-display-name')),
      'North Gate Guard',
    );
    await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(publishCalls, 0);
    expect(find.textContaining('catalog changed'), findsOneWidget);
    expect(find.text('No archetype selected'), findsOneWidget);
    expect(find.byKey(const Key('revision3-npc-wizard')), findsOneWidget);
  });

  testWidgets('locks the wizard after exact-current verification is lost', (
    tester,
  ) async {
    await _setSurface(tester);
    await _openWizard(
      tester,
      loadCatalog: (_) async => _catalog(),
      chooseArchetype: (_, _) async => _asghanCatalogId,
      publish: ({required gameRoot, required input}) async =>
          throw const Revision3NpcDraftRequiresReopenException(),
    );
    await tester.pumpAndSettle();
    await _fillAndSelect(tester);

    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(find.textContaining('reopen the managed project'), findsOneWidget);
    final submit = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-npc-submit')),
    );
    expect(submit.onPressed, isNull);
    expect(find.text('Close'), findsOneWidget);
  });

  testWidgets('requires a fresh wizard after the project checkpoint changed', (
    tester,
  ) async {
    await _setSurface(tester);
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async => _catalog(),
      chooseArchetype: (_, _) async => _asghanCatalogId,
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        throw const Revision3NpcDraftStaleCheckpointException();
      },
    );
    await tester.pumpAndSettle();
    await _fillAndSelect(tester);

    await tester.tap(find.byKey(const Key('revision3-npc-submit')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(
      find.textContaining('changed while this wizard was open'),
      findsOneWidget,
    );
    final submit = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-npc-submit')),
    );
    expect(submit.onPressed, isNull);
  });

  testWidgets('catalog failure can retry without authoring anything', (
    tester,
  ) async {
    await _setSurface(tester);
    var loadCalls = 0;
    var publishCalls = 0;
    await _openWizard(
      tester,
      loadCatalog: (_) async {
        loadCalls += 1;
        if (loadCalls == 1) throw StateError('offline');
        return _catalog();
      },
      chooseArchetype: (_, _) async => _asghanCatalogId,
      publish: ({required gameRoot, required input}) async {
        publishCalls += 1;
        return _publication();
      },
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('could not be refreshed'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-npc-catalog-retry')));
    await tester.pumpAndSettle();

    expect(loadCalls, 2);
    expect(publishCalls, 0);
    expect(find.byKey(const Key('revision3-npc-display-name')), findsOneWidget);
  });
}

Future<void> _setSurface(WidgetTester tester) =>
    tester.binding.setSurfaceSize(const Size(1280, 900));

Future<void> _openWizard(
  WidgetTester tester, {
  required Revision3NpcCatalogLoader loadCatalog,
  required Revision3NpcDraftPublisher publish,
  required Revision3NpcArchetypeChooser chooseArchetype,
  String? initialCatalogId,
  ValueChanged<Revision3NpcDraftPublication?>? onResult,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: FilledButton(
              onPressed: () async {
                final result = await showDialog<Revision3NpcDraftPublication>(
                  context: context,
                  builder: (_) => Revision3NpcWizardDialog(
                    gameRoot: _gameRoot,
                    loadCatalog: loadCatalog,
                    publish: publish,
                    chooseArchetype: chooseArchetype,
                    initialCatalogId: initialCatalogId,
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
  await tester.tap(find.text('Open'));
  await tester.pump();
}

Future<void> _fillAndSelect(WidgetTester tester) async {
  await tester.enterText(
    find.byKey(const Key('revision3-npc-display-name')),
    'North Gate Guard',
  );
  await tester.tap(find.byKey(const Key('revision3-npc-choose-archetype')));
  await tester.pumpAndSettle();
}

Revision3NpcCatalog _catalog({bool onlyViper = false}) => Revision3NpcCatalog(
  choices: [
    if (!onlyViper)
      Revision3NpcCatalogChoice(
        catalogId: _asghanCatalogId,
        displayName: 'Asghan',
      ),
    Revision3NpcCatalogChoice(catalogId: _viperCatalogId, displayName: 'Viper'),
  ],
);

Revision3NpcDraftPublication _publication() => Revision3NpcDraftPublication(
  projectId: _projectId,
  projectRevision: 8,
  npcId: _npcId,
  scriptModuleId: _moduleId,
);
