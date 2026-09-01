import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/ui/design/app_theme.dart';

import 'support/l10n_test_app.dart';

CharacterRow _row(String id) => CharacterRow(
  globalId: id,
  uniqueName: id.split('-').first,
  isDead: false,
  hasInventory: false,
  hasKnowledge: false,
  hasEvents: false,
);

/// The mercenary Wolf is a man; the sixty-six others are wolves. Both resolve
/// to the same display name, and the catalog is what tells them apart.
final _catalog = CharacterCategoryCatalog(const {
  'nc_sld_wolf_701': CharacterCategory.human,
  'creature_wolf': CharacterCategory.creature,
}, const {});

void main() {
  Widget pump(List<CharacterRow> rows) {
    Future<CharacterIndexPage> load() async =>
        CharacterIndexPage(characters: rows, total: rows.length);
    return ProviderScope(
      child: MaterialApp(
        locale: const Locale('en'),
        theme: buildGoresaveTheme(),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: SizedBox(
            width: 380,
            height: 700,
            child: CharacterMasterList(
              selected: const Actor.player(),
              onSelect: (_) {},
              load: load,
              reloadKey: 'k',
              locCatalog: const {
                'nc_sld_wolf_701': {'english': 'Wolf'},
                'wolf': {'english': 'Wolf'},
              },
              lang: const GameLang('en', 'English', Locale('en'), kEnglishLocSets),
              categories: _catalog,
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('a man and the monsters that share his name are separate rows', (
    tester,
  ) async {
    // Without the person in the grouping key the mercenary was folded in with
    // the pack: one row reading "Wolf (4)", wearing his face, that opened to
    // show three animals under it.
    await tester.pumpWidget(
      pump([
        _row('NC_SLD_Wolf_701-WP_1'),
        for (var i = 1; i <= 3; i++) _row('Wolf-OW_PATH_$i'),
      ]),
    );
    await tester.pumpAndSettle();

    // The three animals fold into one group; the man keeps his own plain row.
    expect(find.text('Wolf (3)'), findsOneWidget);
    expect(find.text('Wolf (4)'), findsNothing);
    expect(find.text('Wolf'), findsOneWidget);
  });

  testWidgets('one species stays one row even where the catalog cannot place '
      'it', (tester) async {
    // Whether a creature resolves at all depends on the shape of its save id.
    // Splitting on the catalog's raw kind therefore gave the same animal two
    // rows — "Wolf (9)" beside "Wolf (57)", and the same for the blood flies
    // and the meatbugs.
    await tester.pumpWidget(
      pump([
        // Spawned at a waypoint, which is the form the catalog can place...
        for (var i = 1; i <= 3; i++) _row('Wolf-WP_PATH_$i'),
        // ...and the form it cannot.
        for (var i = 1; i <= 2; i++) _row('Wolf-OW_PATH_${i}_WP-1'),
      ]),
    );
    await tester.pumpAndSettle();

    expect(find.text('Wolf (5)'), findsOneWidget);
    expect(find.text('Wolf (3)'), findsNothing);
    expect(find.text('Wolf (2)'), findsNothing);
  });
}
