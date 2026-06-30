import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/ui/actor_selector.dart';
import 'package:goresave/loc/game_lang.dart';

import 'support/l10n_test_app.dart';

NpcActor _npc(String id) =>
    NpcActor(id: id, isDead: false, hp: 80, maxHp: 80);

void main() {
  testWidgets('ActorSelector shows Player on top and lists NPCs', (
    tester,
  ) async {
    // Fake loader returning a fixed page of NPCs. No core is touched.
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Lizard-WP_A'), _npc('Herek-WP_B')],
        total: 2,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              // Empty catalog → the title falls back to the prettified character
              // key (id prefix before '-'); the raw id stays as the subtitle.
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Player pinned on top.
    expect(find.text('Player'), findsOneWidget);
    // NPC titles are the prettified character keys.
    expect(find.text('Lizard'), findsOneWidget);
    expect(find.text('Herek'), findsOneWidget);
    // Raw GlobalIds retained as subtitles.
    expect(find.text('Lizard-WP_A'), findsOneWidget);
    expect(find.text('Herek-WP_B'), findsOneWidget);
  });

  testWidgets('tapping an NPC selects it; tapping Player selects player', (
    tester,
  ) async {
    final selections = <Actor>[];

    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Lizard-WP_A'), _npc('Herek-WP_B')],
        total: 2,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: selections.add,
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Tap the NPC tile via its title (the prettified character key).
    await tester.tap(find.text('Lizard'));
    await tester.pump();
    expect(selections.last.isPlayer, isFalse);
    expect(selections.last.id, 'Lizard-WP_A');
    expect(selections.last.name, 'Lizard');

    await tester.tap(find.text('Player'));
    await tester.pump();
    expect(selections.last.isPlayer, isTrue);
  });

  testWidgets('a dead NPC shows the skull/death avatar and NO badges', (
    tester,
  ) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [
          NpcActor(
            id: 'Diego',
            isDead: true,
            hp: 0,
            maxHp: 80,
          ),
        ],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Death is encoded in the LEADING avatar (the "dangerous"/skull icon), and
    // the live face icon is gone for a dead (killed) NPC.
    expect(find.byIcon(Icons.dangerous), findsOneWidget);
    expect(find.byIcon(Icons.face_outlined), findsNothing);
    // The dead / XP trailing badges were removed entirely.
    expect(find.text('dead'), findsNothing);
    expect(find.text('XP'), findsNothing);
  });

  testWidgets('an alive NPC keeps the normal face avatar (no skull)', (
    tester,
  ) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Lizard-WP_A')],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Alive NPC → the unchanged face avatar, never the death icon.
    expect(find.byIcon(Icons.face_outlined), findsOneWidget);
    expect(find.byIcon(Icons.dangerous), findsNothing);
  });

  testWidgets('a short id subtitle renders fully (no premature ellipsis)', (
    tester,
  ) async {
    // A short id that easily fits the tile width: with the trailing badges
    // removed, nothing reserves width, so it renders WITHOUT the overflow
    // ellipsis (didExceedMaxLines is false).
    const shortId = 'Diego-1';
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc(shortId)],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final subtitle = tester.widget<Text>(find.text(shortId));
    expect(subtitle.data, shortId);
    final paragraph = find
        .descendant(of: find.text(shortId), matching: find.byType(RichText))
        .evaluate()
        .single
        .renderObject! as RenderParagraph;
    expect(paragraph.didExceedMaxLines, isFalse);
  });

  testWidgets('the GlobalId subtitle spans the full tile width (no badges)', (
    tester,
  ) async {
    // Regression for #1: removing the trailing badges must let the subtitle use
    // the FULL available width. A long id is laid out across (nearly) the whole
    // tile rather than being squeezed by a trailing widget — so the subtitle's
    // painted width is close to the tile width minus the leading avatar gutter.
    const longId = 'Herek-WP_OM_TUNNEL_ME_01';
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc(longId)],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Scope to the NPC tile (the player tile is also a ListTile).
    final npcTile = find.ancestor(
      of: find.text(longId),
      matching: find.byType(ListTile),
    );
    final tileWidth = tester.getSize(npcTile).width;
    final subtitleWidth = tester.getSize(find.text(longId)).width;
    // The subtitle uses most of the tile width (only the ~40px leading avatar +
    // paddings are subtracted). With a trailing badge it would have been
    // markedly narrower; assert it now exceeds 70% of the tile width.
    expect(subtitleWidth, greaterThan(tileWidth * 0.7));
  });

  testWidgets('Player tile renders above the NPC search field', (tester) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Lizard')],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The Player tile sits vertically above the "Search NPCs" field.
    final playerTop = tester.getTopLeft(find.text('Player')).dy;
    final searchTop = tester
        .getTopLeft(find.widgetWithText(TextField, 'Search NPCs'))
        .dy;
    expect(playerTop, lessThan(searchTop));
  });

  testWidgets('NPC tile resolves a localized name and keeps the id as subtitle',
      (tester) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Herek-WP_OM_TUNNEL_ME_01')],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    // Catalog keyed by the character key (prefix before '-', lowercased).
    final catalog = <String, Map<String, String>>{
      'herek': {'english': 'Herek'},
    };

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: catalog,
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Resolved localized name as the title.
    expect(find.text('Herek'), findsOneWidget);
    // Raw GlobalId retained as the subtitle.
    expect(find.text('Herek-WP_OM_TUNNEL_ME_01'), findsOneWidget);
  });

  testWidgets('unresolved NPC id falls back to a prettified name', (
    tester,
  ) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Creature_Meatbug-WP_OM_TUNNEL_01')],
        total: 1,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              // Empty catalog → falls back to the prettified character key.
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 'Creature_Meatbug' → prefix stripped + humanized → 'Meatbug'.
    expect(find.text('Meatbug'), findsOneWidget);
    // Raw id retained as subtitle.
    expect(find.text('Creature_Meatbug-WP_OM_TUNNEL_01'), findsOneWidget);
  });

  testWidgets('searching a resolved NAME filters to that NPC', (tester) async {
    // The full list is fetched ONCE (empty query, high limit). The fake ignores
    // the query and always returns the full set, so any filtering happening
    // here is the CLIENT-SIDE id|name predicate under test.
    var calls = 0;
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      calls++;
      return NpcActorsPage(
        npcs: [
          // Raw id's searchable part is the character key 'OC_NSC_01' — it does
          // NOT contain 'Diego'. Only the RESOLVED loc name does.
          _npc('OC_NSC_01-WP_A'),
          _npc('Lizard-WP_B'),
        ],
        total: 2,
        offset: 0,
        limit: limit,
      );
    }

    // Catalog resolves the first NPC's character key to the display name 'Diego'.
    final catalog = <String, Map<String, String>>{
      'oc_nsc_01': {'english': 'Diego'},
    };

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: catalog,
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Helper: NPC tile titles live inside ListTile, so scope the match to
    // ListTile to avoid matching the search field's own text.
    Finder tileText(String s) => find.descendant(
          of: find.byType(ListTile),
          matching: find.text(s),
        );

    // Both NPCs visible before searching.
    expect(tileText('Diego'), findsOneWidget);
    expect(tileText('Lizard'), findsOneWidget);

    // Type a NAME that is NOT a substring of the raw id's searchable part.
    await tester.enterText(find.widgetWithText(TextField, 'Search NPCs'), 'Diego');
    await tester.pumpAndSettle();

    // Filtered to the named NPC; the other drops out.
    expect(tileText('Diego'), findsOneWidget);
    expect(tileText('Lizard'), findsNothing);

    // The full list was fetched ONCE — searching does not re-hit the loader.
    expect(calls, 1);
  });

  testWidgets('searching an id substring still filters', (tester) async {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      return NpcActorsPage(
        npcs: [_npc('Lizard-WP_A'), _npc('Herek-WP_B')],
        total: 2,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 'lizard' is a substring of the raw id 'Lizard-WP_A' (case-insensitive).
    await tester.enterText(find.widgetWithText(TextField, 'Search NPCs'), 'lizard');
    await tester.pumpAndSettle();

    expect(find.text('Lizard'), findsOneWidget);
    expect(find.text('Herek'), findsNothing);
  });

  testWidgets('pagination reflects the FILTERED count', (tester) async {
    // 250 NPCs total; 3 of them resolve to names containing 'Diego'.
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      final npcs = <NpcActor>[
        for (var i = 0; i < 250; i++) _npc('Filler_$i-WP'),
      ];
      return NpcActorsPage(
        npcs: npcs,
        total: npcs.length,
        offset: 0,
        limit: limit,
      );
    }

    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: fakeLoad,
              reloadKey: 'k',
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Unfiltered: 250 rows → "1–100 of 250" (page size 100).
    expect(find.text('1–100 of 250'), findsOneWidget);

    // Filter to a single filler key — exactly one match.
    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'Filler_42-',
    );
    await tester.pumpAndSettle();

    // Pagination recomputes over the FILTERED list (1 of 1).
    expect(find.text('1–1 of 1'), findsOneWidget);
  });

  // ---------------------------------------------------------------------------
  // Bug #6: when the inspected save changes (reloadKey changes), the selector
  // must re-invoke the loader and replace the stale NPC list — the widget is
  // kept alive across save switches, so without this the previous file's NPCs
  // (and dead/bounty badges) would linger.
  // ---------------------------------------------------------------------------
  testWidgets('changing reloadKey reloads the NPC list and resets state', (
    tester,
  ) async {
    var calls = 0;
    // A SINGLE stable loader (its identity never changes across rebuilds, like
    // the real `loadAllNpcActors` method reference) that returns whatever the
    // current `nextNpcs` is — so only a reloadKey change can drive the reload.
    var nextNpcs = <NpcActor>[_npc('Lizard-WP_A')];
    Future<NpcActorsPage> stableLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async {
      calls++;
      return NpcActorsPage(
        npcs: nextNpcs,
        total: nextNpcs.length,
        offset: 0,
        limit: limit,
      );
    }

    Widget build(Object reloadKey) {
      return wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 320,
            height: 600,
            child: ActorSelector(
              selected: const Actor.player(),
              onSelect: (_) {},
              loadNpcs: stableLoad,
              reloadKey: reloadKey,
              locCatalog: const {},
              lang: kGameLangs.first,
            ),
          ),
        ),
      );
    }

    // Save A: shows Lizard.
    await tester.pumpWidget(build('save-A'));
    await tester.pumpAndSettle();
    expect(calls, 1);
    expect(find.text('Lizard'), findsOneWidget);

    // Switch to save B (new reloadKey only): the same loader re-runs and the
    // list swaps to the new save's NPCs.
    nextNpcs = <NpcActor>[_npc('Herek-WP_B')];
    await tester.pumpWidget(build('save-B'));
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('Herek'), findsOneWidget);
    // The previous save's NPC is gone (state reset, not appended).
    expect(find.text('Lizard'), findsNothing);
  });
}
