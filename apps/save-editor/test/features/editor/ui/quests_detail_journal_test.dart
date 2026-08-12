import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/ui/progression_panel.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import '../../../support/l10n_test_app.dart';

void main() {
  testWidgets(
    'QuestsDetail shows localized journal sections and nested objectives',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _QuestJournalNotifier();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            locCatalogProvider.overrideWith((ref) async => _questCatalog),
          ],
          child: wrapWithL10n(
            Builder(
              builder: (context) => Scaffold(
                body: QuestsDetail(
                  notifier: notifier,
                  editable: false,
                  reloadKey: const SaveInspection(
                    format: 'G1R',
                    path: r'C:\tmp\saves\G1R-006.sav',
                    size: 1,
                    sha1: 'quest-journal-test',
                    raw: {},
                  ),
                  theme: Theme.of(context),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // The sidebar mirrors the five localized in-game sections. Raw save
      // groups from the old flat picker are not exposed as navigation items.
      expect(find.text('Old Camp (1)'), findsOneWidget);
      expect(find.text('New Camp (1)'), findsOneWidget);
      expect(find.text('Swamp Camp (1)'), findsOneWidget);
      expect(find.text('The Colony (1)'), findsOneWidget);
      expect(find.text('Completed (1)'), findsOneWidget);
      expect(find.textContaining('OldCamp ('), findsNothing);
      expect(find.textContaining('ValleyOfMines ('), findsNothing);
      expect(find.textContaining('Tutorials ('), findsNothing);

      // A localized objective is hidden below its localized main quest until
      // that quest is expanded instead of appearing as a second flat row.
      expect(find.text('Trial of Trust'), findsOneWidget);
      expect(find.text("Collect Ian's list"), findsNothing);
      expect(find.byType(ExpansionTile), findsOneWidget);
      await tester.tap(find.byIcon(Icons.expand_more));
      await tester.pumpAndSettle();
      expect(find.text("Collect Ian's list"), findsOneWidget);
      expect(find.byIcon(Icons.subdirectory_arrow_right), findsOneWidget);

      // Tutorials may have complete quest localization, but belong to the
      // glossary and must not leak into either the sidebar or quest tree.
      expect(find.text('Using the map'), findsNothing);
      expect(find.text('Map tutorial text.'), findsNothing);
    },
  );
}

class _QuestJournalNotifier extends EditorNotifier {
  _QuestJournalNotifier() : super(_NoopCore(), saveDir: r'C:\tmp\saves');

  @override
  Future<ProgressionQuestPage> loadProgressionQuests({
    String query = '',
    String? state,
    String? group,
    int offset = 0,
    int limit = 100,
    String? path,
  }) async => ProgressionQuestPage(
    quests: _quests,
    total: _quests.length,
    offset: offset,
    limit: limit,
  );
}

const _questCatalog = <String, Map<String, String>>{
  'quest-oldcamp_occhapter1_bringlist-name': {'english': 'Trial of Trust'},
  'quest-oldcamp_occhapter1_bringlist-description': {
    'english': 'Bring Diego the list.',
  },
  'quest-oldcamp_occhapter1_bringlist_bringlist_obj_getlist-name': {
    'english': "Collect Ian's list",
  },
  'quest-newcamp_ncchapter1_damlurker-name': {'english': 'The dam lurker'},
  'quest-newcamp_ncchapter1_damlurker-description': {
    'english': 'Kill the lurker at the dam.',
  },
  'quest-swampcamp_scchapter1_harvest-name': {'english': 'The weed harvest'},
  'quest-swampcamp_scchapter1_harvest-description': {
    'english': 'Help with the swampweed harvest.',
  },
  'quest-valleyofmines_findnek-name': {'english': 'Find Nek'},
  'quest-valleyofmines_findnek-description': {
    'english': 'Find the missing guard.',
  },
  'quest-oldmine_finished-name': {'english': 'A finished quest'},
  'quest-oldmine_finished-description': {'english': 'This quest is over.'},
  'quest-tutorials_tut_map-name': {'english': 'Using the map'},
  'quest-tutorials_tut_map-description': {'english': 'Map tutorial text.'},
};

final _quests = <ProgressionQuest>[
  _quest('Quest_OldCamp', 'OldCamp', 'EQuestState::Running'),
  _quest('Quest_OldCamp_OCCHAPTER1', 'OldCamp', 'EQuestState::Running'),
  _quest(
    'Quest_OldCamp_OCCHAPTER1_BRINGLIST',
    'OldCamp',
    'EQuestState::Running',
  ),
  _quest(
    'Quest_OldCamp_OCCHAPTER1_BRINGLIST_BRINGLIST_OBJ_GETLIST',
    'OldCamp',
    'EQuestState::Succeeded',
  ),
  _quest(
    'Quest_NewCamp_NCCHAPTER1_DAMLURKER',
    'NewCamp',
    'EQuestState::Running',
  ),
  _quest(
    'Quest_SwampCamp_SCCHAPTER1_HARVEST',
    'SwampCamp',
    'EQuestState::Running',
  ),
  _quest(
    'Quest_ValleyOfMines_FINDNEK',
    'ValleyOfMines',
    'EQuestState::Running',
  ),
  _quest('Quest_OldMine_FINISHED', 'OldMine', 'EQuestState::Succeeded'),
  _quest('Quest_Tutorials', 'Tutorials', 'EQuestState::Running'),
  _quest('Quest_Tutorials_Tut_Map', 'Tutorials', 'EQuestState::Succeeded'),
];

ProgressionQuest _quest(String id, String group, String state) =>
    ProgressionQuest(
      questClass: '/Script/Angelscript.$id',
      id: id,
      group: group,
      name: id.substring('Quest_'.length),
      currentState: state,
      statePath: [
        'QuestDataByClass',
        '{/Script/Angelscript.$id}',
        'CurrentState',
      ],
      writable: true,
    );

class _NoopCore implements GoresaveCoreService {
  @override
  String get description => 'quest-journal-widget-test';

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
