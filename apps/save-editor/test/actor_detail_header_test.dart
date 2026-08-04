import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/ui/actor_detail_header.dart';
import 'package:goresave/loc/game_lang.dart';

import 'support/l10n_test_app.dart';

void main() {
  testWidgets('NPC header always shows the GlobalId when ids are disabled', (
    tester,
  ) async {
    const id = 'Herek-WP_OM_TUNNEL_ME_01';
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: ActorDetailHeader(
            actor: const Actor.npc(id: id, name: 'Herek', uniqueName: 'Herek'),
            locCatalog: const {
              'herek': {'english': 'Herek'},
            },
            lang: kGameLangs.first,
            showObjectIds: false,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Herek'), findsOneWidget);
    expect(find.widgetWithText(SelectableText, id), findsOneWidget);
  });

  testWidgets(
    'NPC header shows the resolved name + the FULL GlobalId when enabled',
    (tester) async {
      const longId = 'Herek-WP_OM_TUNNEL_ME_01';
      final catalog = <String, Map<String, String>>{
        'herek': {'english': 'Herek'},
      };

      await tester.pumpWidget(
        wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 600,
              child: ActorDetailHeader(
                actor: const Actor.npc(
                  id: longId,
                  name: 'Herek',
                  uniqueName: 'Herek',
                ),
                locCatalog: catalog,
                lang: kGameLangs.first,
                showObjectIds: true,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Resolved (loc catalog) display name shown prominently.
      expect(find.text('Herek'), findsOneWidget);
      // The FULL GlobalId is present in the tree as SELECTABLE text. A
      // SelectableText with no maxLines cap wraps rather than ellipsizing, so the
      // whole id is always readable.
      final idFinder = find.widgetWithText(SelectableText, longId);
      expect(idFinder, findsOneWidget);
      final id = tester.widget<SelectableText>(idFinder);
      expect(id.data, longId);
      expect(id.maxLines, isNull);
    },
  );

  testWidgets('NPC header resolves a prettified name when catalog misses', (
    tester,
  ) async {
    const id = 'Creature_Meatbug-WP_OM_TUNNEL_01';
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 600,
            child: ActorDetailHeader(
              actor: const Actor.npc(
                id: id,
                name: 'ignored',
                uniqueName: 'Creature_Meatbug',
              ),
              locCatalog: const {},
              lang: kGameLangs.first,
              showObjectIds: true,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Same prettify fallback as the list: 'Creature_Meatbug' → 'Meatbug'.
    expect(find.text('Meatbug'), findsOneWidget);
    // Full id still visible.
    expect(find.text(id), findsOneWidget);
  });

  testWidgets('player header shows "Player" and NO GlobalId', (tester) async {
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 600,
            child: ActorDetailHeader(
              actor: const Actor.player(),
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Localized "Player" (tabPlayer).
    expect(find.text('Player'), findsOneWidget);
    // The player has no GlobalId → no SelectableText id field.
    expect(find.byType(SelectableText), findsNothing);
  });

  testWidgets('orphan shows its real knowledge key, never the sentinel', (
    tester,
  ) async {
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: ActorDetailHeader(
            actor: const Actor.npc(
              id: 'orphan:ST_VLK_Mud_Sleeper',
              name: 'Mud',
              uniqueName: 'ST_VLK_Mud_Sleeper',
            ),
            locCatalog: const {},
            lang: kGameLangs.first,
            showObjectIds: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('ST_VLK_Mud_Sleeper'), findsOneWidget);
    expect(find.text('orphan:ST_VLK_Mud_Sleeper'), findsNothing);
  });
}
