import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_dialog.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _currentParentId = 'catalog-parent-secret';
const _currentGiverId = 'catalog-giver-secret';

void main() {
  testWidgets(
    'prefills friendly context, hides identities and disables no-op',
    (tester) async {
      await _open(tester, catalogs: [_catalog('a')]);

      expect(find.text('Edit Quest details'), findsOneWidget);
      expect(find.text('Find Homer'), findsOneWidget);
      expect(find.text('Chapter Two'), findsOneWidget);
      expect(find.text('Asghan'), findsOneWidget);
      expect(find.textContaining(_currentParentId), findsNothing);
      expect(find.textContaining(_currentGiverId), findsNothing);
      expect(find.textContaining('UQuest_'), findsNothing);
      expect(find.textContaining('OM_GRD_'), findsNothing);
      expect(find.textContaining(revision3QuestOutlineQuestId), findsNothing);
      expect(find.textContaining('offline project draft only'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-quest-context-save')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'saves one reviewed description edit after fresh catalog reload',
    (tester) async {
      Revision3QuestContextEditTechnicalPlan? received;
      var resultSeen = false;
      await _open(
        tester,
        catalogs: [_catalog('a'), _catalog('a')],
        publish: (plan) {
          received = plan;
          return _publication();
        },
        onResult: (result) => resultSeen = result != null,
      );

      await tester.enterText(
        find.byKey(const Key('revision3-quest-context-description')),
        'Find Homer and report back safely.',
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
      await tester.pumpAndSettle();

      expect(received?.description, 'Find Homer and report back safely.');
      expect(received?.parentCatalogId, _currentParentId);
      expect(received?.giverCatalogId, _currentGiverId);
      expect(resultSeen, isTrue);
      expect(
        find.byKey(const Key('revision3-quest-context-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('hotfix drift clears selections and requires review', (
    tester,
  ) async {
    var publishes = 0;
    await _open(
      tester,
      catalogs: [_catalog('a'), _catalog('b'), _catalog('b')],
      publish: (plan) {
        publishes++;
        return _publication();
      },
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-context-description')),
      'Find Homer and report back safely.',
    );
    await tester.pump();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-context-save')),
          )
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
    await tester.pumpAndSettle();

    expect(publishes, 0);
    expect(find.textContaining('game choices changed'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-context-save')),
          )
          .onPressed,
      isNull,
    );
    expect(find.textContaining(_currentParentId), findsNothing);
    expect(find.textContaining('UQuest_'), findsNothing);

    final parentPicker = find.byType(DropdownButtonFormField<String>).first;
    await tester.tap(parentPicker);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Chapter Two').last);
    await tester.pumpAndSettle();
    expect(find.textContaining('game choices changed'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-context-save')),
          )
          .onPressed,
      isNull,
    );

    final giverPicker = find.byType(DropdownButtonFormField<String>).last;
    await tester.ensureVisible(giverPicker);
    await tester.pumpAndSettle();
    await tester.tap(giverPicker);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Asghan').last);
    await tester.pumpAndSettle();
    expect(find.textContaining('game choices changed'), findsNothing);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-context-save')),
          )
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
    await tester.pumpAndSettle();
    expect(publishes, 1);
    expect(
      find.byKey(const Key('revision3-quest-context-dialog')),
      findsNothing,
    );
  });

  testWidgets('dirty outside dismiss is blocked and cancel confirms discard', (
    tester,
  ) async {
    await _open(tester, catalogs: [_catalog('a')]);
    await tester.enterText(
      find.byKey(const Key('revision3-quest-context-description')),
      'Find Homer and report back safely.',
    );
    await tester.pump();

    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-context-dialog')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const Key('revision3-quest-context-cancel')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-context-discard-dialog')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-context-keep-editing')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-context-dialog')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const Key('revision3-quest-context-cancel')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-quest-context-discard')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-quest-context-dialog')),
      findsNothing,
    );
  });

  testWidgets('missing current mapping locks editor without guessing', (
    tester,
  ) async {
    await _open(tester, catalogs: [_catalog('a', includeCurrent: false)]);

    expect(find.textContaining('cannot guess a replacement'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-quest-context-description')),
      findsNothing,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-quest-context-save')),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets('native game input errors point to installation recovery', (
    tester,
  ) async {
    await _open(
      tester,
      catalogs: [_catalog('a'), _catalog('a')],
      publish: (_) => throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
        code: 'INPUT_CHANGED',
        message: 'input changed',
      ),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-context-description')),
      'Find Homer and report back safely.',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('configured game installation changed'),
      findsOneWidget,
    );
    expect(find.textContaining('Verify it in Settings'), findsOneWidget);
    expect(find.textContaining('Review the description'), findsNothing);
  });

  testWidgets('native request errors point to editable Quest fields', (
    tester,
  ) async {
    await _open(
      tester,
      catalogs: [_catalog('a'), _catalog('a')],
      publish: (_) => throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
        code: 'REQUEST_INVALID',
        message: 'request invalid',
      ),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-quest-context-description')),
      'Find Homer and report back safely.',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-quest-context-save')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining(
        'Review the description, Quest family, and giver before saving.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('Verify it in Settings'), findsNothing);
  });
}

Future<void> _open(
  WidgetTester tester, {
  required List<Revision3QuestCatalog> catalogs,
  Revision3QuestContextEditPublication Function(
    Revision3QuestContextEditTechnicalPlan plan,
  )?
  publish,
  ValueChanged<Revision3QuestContextEditPublication?>? onResult,
}) async {
  final fixture = Revision3QuestOutlineFixture();
  final index = fixture.contentIndex();
  var catalogIndex = 0;
  final service = Revision3QuestContextAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
          required expectedParentRuntimeClass,
          required expectedGiverRuntimeUniqueName,
        }) async => AuthoringRevision3QuestContextSeed.forProject(
          currentProjectJson: fixture.projectJson,
          questId: questId,
          expectedQuestRevision: expectedQuestRevision,
          expectedModuleId: expectedModuleId,
          expectedModuleRevision: expectedModuleRevision,
          expectedParentRuntimeClass: expectedParentRuntimeClass,
          expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
        ),
    loadCatalog: (_) async => catalogs[catalogIndex++],
    publishTechnicalPlan: ({required gameRoot, required plan}) async =>
        publish?.call(plan) ?? _publication(),
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: FilledButton(
            onPressed: () async {
              final result =
                  await showDialog<Revision3QuestContextEditPublication>(
                    context: context,
                    builder: (_) => Revision3QuestContextEditDialog(
                      index: index,
                      quest: index.entityById(revision3QuestOutlineQuestId)!,
                      gameRoot: _gameRoot,
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

Revision3QuestCatalog _catalog(
  String sealDigit, {
  bool includeCurrent = true,
}) => Revision3QuestCatalog(
  parents: [
    if (includeCurrent)
      Revision3QuestParentChoice(
        catalogId: _currentParentId,
        displayName: 'Chapter Two',
        runtimeClass: 'UQuest_SwampCamp_SCChapter2',
        catalogLayer: 'base-game.quest-parent.v1',
        authoringSelector: 'SwampCamp_SCChapter2',
        sourceSeal: _sourceSeal(11, '1'),
      ),
    Revision3QuestParentChoice(
      catalogId: revision3QuestContextParentCatalogId,
      displayName: 'Chapter Three',
      runtimeClass: revision3QuestContextParentRuntimeClass,
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter3',
      sourceSeal: _sourceSeal(11, '1'),
    ),
  ],
  givers: [
    if (includeCurrent)
      Revision3QuestGiverChoice(
        catalogId: _currentGiverId,
        displayName: 'Asghan',
        runtimeUniqueName: 'OM_GRD_Asghan_263',
        catalogLayer: 'base-game.npc.v1',
        authoringSelector: 'OM_GRD_Asghan_263',
        sourceSeal: _sourceSeal(12, '2'),
      ),
    Revision3QuestGiverChoice(
      catalogId: revision3QuestContextGiverCatalogId,
      displayName: 'Viper',
      runtimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: revision3QuestContextGiverRuntimeUniqueName,
      sourceSeal: _sourceSeal(12, '2'),
    ),
  ],
  catalogSeal: AuthoringDraftContentSeal.fromJson({
    'byte_len': 2048,
    'sha256': List.filled(64, sealDigit).join(),
  }),
  generationExecutableSeal: AuthoringDraftContentSeal.fromJson({
    'byte_len': 171698176,
    'sha256': List.filled(64, 'a').join(),
  }),
);

AuthoringDraftContentSeal _sourceSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson({
      'byte_len': bytes,
      'sha256': List.filled(64, digit).join(),
    });

Revision3QuestContextEditPublication _publication() =>
    Revision3QuestContextEditPublication(
      projectId: revision3QuestOutlineProjectId,
      projectRevision: 8,
      questId: revision3QuestOutlineQuestId,
      moduleId: revision3QuestOutlineModuleId,
      questRevision: 5,
      moduleRevision: 6,
    );
