import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/glossary_models.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/ui/glossary_panel.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import '../../../support/l10n_test_app.dart';

void main() {
  testWidgets(
    'tutorial glossary counts unlocked gates and queues state edits',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1050, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _TutorialGlossaryNotifier();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            locCatalogProvider.overrideWith(
              (ref) async => const <String, Map<String, String>>{},
            ),
          ],
          child: wrapWithL10n(
            Builder(
              builder: (context) => Scaffold(
                body: GlossaryDetail(
                  notifier: notifier,
                  editable: true,
                  reloadKey: const SaveInspection(
                    format: 'G1R',
                    path: r'C:\tmp\saves\G1R-006.sav',
                    size: 1,
                    sha1: 'tutorial-test',
                    raw: {},
                  ),
                  theme: Theme.of(context),
                  npcCatalogLoader: () async => const [],
                  segmentTextCatalogLoader: () async => const {},
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final tutorialTile = find.widgetWithText(ListTile, 'Tutorials');
      expect(tutorialTile, findsOneWidget);
      expect(
        (tester.widget<ListTile>(tutorialTile).trailing as Text).data,
        '10',
      );

      await tester.tap(tutorialTile);
      await tester.pumpAndSettle();

      expect(
        find.text(
          'These rows control saved tutorial unlock gates. A gate does not '
          'necessarily map one-to-one to an individual in-game tutorial page.',
        ),
        findsOneWidget,
      );
      expect(find.text('10 of 15 tutorial gates unlocked'), findsOneWidget);
      expect(find.text('Combat basics'), findsOneWidget);
      expect(find.text('Tut_CombatBasics'), findsNothing);

      const firstId = 'Quest_Tutorials_Tut_CombatBasics';
      await tester.tap(find.byKey(const Key('tutorial-state-$firstId')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Failed').last);
      await tester.pumpAndSettle();

      expect(find.text('9 of 15 tutorial gates unlocked'), findsOneWidget);
      expect(find.text('Unsaved change'), findsOneWidget);
      final pending = notifier.pendingEditFor('progression.tutorials');
      expect(pending, isNotNull);
      expect(pending!.edits, [
        {
          'path': 'private.typed.setValue',
          'value': {
            'path': [
              'QuestDataByClass',
              '{/Script/Angelscript.$firstId}',
              'CurrentState',
            ],
            'value': 'EQuestState::Failed',
          },
        },
      ]);

      await tester.tap(find.byTooltip('Reset tutorial changes'));
      await tester.pumpAndSettle();
      expect(find.text('10 of 15 tutorial gates unlocked'), findsOneWidget);
      expect(notifier.pendingEditFor('progression.tutorials'), isNull);

      await tester.tap(find.byKey(const Key('tutorial-state-$firstId')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Failed').last);
      await tester.pumpAndSettle();
      notifier.clearAllPendingEdits();
      await tester.pump();
      expect(find.text('10 of 15 tutorial gates unlocked'), findsOneWidget);
      expect(find.text('Unsaved change'), findsNothing);

      await tester.enterText(
        find.byKey(const Key('tutorial-gate-search')),
        'trading',
      );
      await tester.pump();
      // The last of the 15 save gates remains editable and searchable even
      // though the virtualized list did not initially build its row.
      expect(find.text('Trading'), findsOneWidget);
      expect(find.text('Combat basics'), findsNothing);
      expect(
        tester
            .widget<ListView>(find.byKey(const Key('tutorial-gate-list')))
            .childrenDelegate
            .estimatedChildCount,
        1,
      );
    },
  );
}

class _TutorialGlossaryNotifier extends EditorNotifier {
  _TutorialGlossaryNotifier() : super(_NoopCore(), saveDir: r'C:\tmp\saves');

  @override
  Future<GlossaryPage> loadGlossary() async => const GlossaryPage();

  @override
  Future<ProgressionQuestPage> loadProgressionTutorials({
    int offset = 0,
    int limit = 100,
  }) async => ProgressionQuestPage(
    quests: _tutorialGates,
    total: _tutorialGates.length,
    offset: offset,
    limit: limit,
  );

  @override
  Future<CharacterIndexPage> loadAllCharacters() async =>
      const CharacterIndexPage();

  @override
  Future<NpcActorsPage> loadAllNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async => const NpcActorsPage();
}

const _tutorialIds = <String>[
  'Tut_CombatBasics',
  'Tut_Crafting',
  'Tut_Crime',
  'Tut_Drugs',
  'Tut_Lockpicking',
  'Tut_Magic',
  'Tut_Map',
  'Tut_MeleeCombat',
  'Tut_Navigation',
  'Tut_Perception',
  'Tut_PlayerProgression',
  'Tut_Ranged',
  'Tut_Riding',
  'Tut_Sleep',
  'Tut_Trading',
];

final _tutorialGates = <ProgressionQuest>[
  for (var index = 0; index < _tutorialIds.length; index++)
    ProgressionQuest(
      questClass: '/Script/Angelscript.Quest_Tutorials_${_tutorialIds[index]}',
      id: 'Quest_Tutorials_${_tutorialIds[index]}',
      group: 'Tutorials',
      name: _tutorialIds[index],
      currentState: index < 5
          ? 'EQuestState::Running'
          : index < 10
          ? 'EQuestState::Succeeded'
          : 'EQuestState::Available',
      statePath: [
        'QuestDataByClass',
        '{/Script/Angelscript.Quest_Tutorials_${_tutorialIds[index]}}',
        'CurrentState',
      ],
      writable: true,
    ),
];

class _NoopCore implements GoresaveCoreService {
  @override
  String get description => 'tutorial-glossary-test';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => switch (command) {
    'scan_save_dir' => {
      'ok': true,
      'data': {
        'saveRoot': payload['path'],
        'saves': <Object?>[],
        'profiles': <Object?>[],
      },
    },
    'check_codec' => {
      'ok': true,
      'data': {'canDecompress': true, 'canCompress': true},
    },
    _ => {'ok': true, 'data': <String, Object?>{}},
  };
}
