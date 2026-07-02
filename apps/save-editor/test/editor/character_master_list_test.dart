import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

void main() {
  // With an empty locCatalog, `localizedNpcName` prettifies the character key
  // (the GlobalId prefix before the first '-'). For 'NC_ORG_Lares_801-WP_X' the
  // key is 'NC_ORG_Lares_801' → strip trailing '_801' → humanize the remaining
  // 'NC_ORG_Lares' segments → 'Nc Org Lares' (no prefix in _npcKeyPrefixes is
  // 'NC_', so nothing is stripped from the front).
  const laresDisplay = 'Nc Org Lares';

  testWidgets('renders Player, an NPC row, and the Weitere orphan group', (
    tester,
  ) async {
    const page = CharacterIndexPage(
      characters: [
        CharacterRow(
          globalId: 'NC_ORG_Lares_801-WP_X',
          uniqueName: 'NC_ORG_Lares_801',
          isDead: false,
          hasInventory: true,
          hasKnowledge: true,
          hasEvents: true,
        ),
        CharacterRow(
          globalId: null,
          uniqueName: 'ST_VLK_Mud_Sleeper',
          isDead: false,
          hasInventory: false,
          hasKnowledge: true,
          hasEvents: false,
        ),
      ],
      total: 2,
    );
    Actor? picked;
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        locale: const Locale('de'),
        home: Scaffold(
          body: CharacterMasterList(
            selected: const Actor.player(),
            onSelect: (a) => picked = a,
            load: () async => page,
            reloadKey: 'k1',
            locCatalog: const {},
            lang: gameLangByCode('de'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Player pinned on top (German label).
    expect(find.text('Spieler'), findsOneWidget);
    // Weitere group present because there's an orphan.
    expect(find.text('Weitere'), findsOneWidget);

    // Tap the actor row (found by its resolved, prettified display name).
    await tester.tap(find.text(laresDisplay));
    await tester.pumpAndSettle();
    expect(picked?.id, 'NC_ORG_Lares_801-WP_X');
    expect(picked?.uniqueName, 'NC_ORG_Lares_801');
    expect(picked?.isOrphan, isFalse);

    // Tap the orphan row → an orphan actor carrying the uniqueName.
    // 'ST_VLK_Mud_Sleeper' has no '-', prefix 'ST_' is not stripped → the whole
    // key humanizes to 'St Vlk Mud Sleeper'.
    await tester.tap(find.text('St Vlk Mud Sleeper'));
    await tester.pumpAndSettle();
    expect(picked!.isOrphan, isTrue);
    expect(picked!.uniqueName, 'ST_VLK_Mud_Sleeper');
  });
}
