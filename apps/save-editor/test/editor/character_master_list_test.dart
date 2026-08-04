import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

import '../support/l10n_test_app.dart';

/// A spawned-actor row; the uniqueName defaults to the GlobalId's character key
/// (prefix before the first '-'), mirroring the core's real rows.
CharacterRow _actor(
  String globalId, {
  String? uniqueName,
  bool isDead = false,
  bool hasKnowledge = false,
  bool hasEvents = false,
}) {
  return CharacterRow(
    globalId: globalId,
    uniqueName: uniqueName ?? globalId.split('-').first,
    isDead: isDead,
    hasInventory: false,
    hasKnowledge: hasKnowledge,
    hasEvents: hasEvents,
  );
}

/// English-locale harness for the behavior tests (retargeted from the retired
/// ActorSelector's suite, which asserted English labels like 'Player').
Widget _pump({
  required Future<CharacterIndexPage> Function() load,
  Actor selected = const Actor.player(),
  void Function(Actor)? onSelect,
  Object reloadKey = 'k',
  Map<String, Map<String, String>> locCatalog = const {},
  bool showObjectIds = true,
}) {
  return wrapWithL10n(
    Scaffold(
      body: SizedBox(
        width: 380,
        height: 600,
        child: CharacterMasterList(
          selected: selected,
          onSelect: onSelect ?? (_) {},
          load: load,
          reloadKey: reloadKey,
          locCatalog: locCatalog,
          lang: kGameLangs.first,
          showObjectIds: showObjectIds,
        ),
      ),
    ),
  );
}

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

  // ---------------------------------------------------------------------------
  // Player/Hero de-duplication: the save carries a REAL "Hero" actor row (the
  // player's avatar). The pinned Player row represents it, so the NPC section
  // must exclude it — otherwise the player would be listed twice.
  // ---------------------------------------------------------------------------
  testWidgets(
    'the save\'s own Hero actor row is excluded from the NPC section',
    (tester) async {
      final page = CharacterIndexPage(
        characters: [
          // The real hero row as emitted by `private.characters.list`.
          _actor(
            'Hero',
            uniqueName: 'Hero',
            hasKnowledge: true,
            hasEvents: true,
          ),
          _actor('Lizard-WP_A'),
          const CharacterRow(
            globalId: null,
            uniqueName: 'ST_VLK_Mud_Sleeper',
            isDead: false,
            hasInventory: false,
            hasKnowledge: true,
            hasEvents: false,
          ),
        ],
        total: 3,
      );
      await tester.pumpWidget(_pump(load: () async => page));
      await tester.pumpAndSettle();

      // Pinned Player + the normal NPC + the orphan are all present.
      expect(find.text('Player'), findsOneWidget);
      expect(find.text('Lizard'), findsOneWidget);
      expect(find.text('St Vlk Mud Sleeper'), findsOneWidget);
      // English-locale harness → the orphan group header is the localized
      // 'Other' (the German suite above pins 'Weitere' for the same header).
      expect(find.text('Other'), findsOneWidget);
      // No 'Hero' NPC row: neither its resolved title nor its id subtitle
      // render (both would be the text 'Hero').
      expect(find.text('Hero'), findsNothing);
      // The hero row is excluded from the actor COUNT too (not merely hidden):
      // pagination shows one actor, the Lizard.
      expect(find.text('1–1 of 1'), findsOneWidget);
    },
  );

  // ---------------------------------------------------------------------------
  // Retargeted from the retired ActorSelector's suite: the master list inherits
  // its structure (pinned Player, id subtitles, dead/alive avatars, search over
  // resolved names, client-side pagination, reloadKey reset), so the behavioral
  // coverage moves here.
  // ---------------------------------------------------------------------------

  testWidgets('shows Player on top and lists actors with id subtitles', (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [_actor('Lizard-WP_A'), _actor('Herek-WP_B')],
          total: 2,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Player pinned on top.
    expect(find.text('Player'), findsOneWidget);
    // Actor titles are the prettified character keys.
    expect(find.text('Lizard'), findsOneWidget);
    expect(find.text('Herek'), findsOneWidget);
    // Raw GlobalIds retained as subtitles.
    expect(find.text('Lizard-WP_A'), findsOneWidget);
    expect(find.text('Herek-WP_B'), findsOneWidget);
  });

  testWidgets('hides actor and orphan ids while keeping readable names', (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        showObjectIds: false,
        load: () async => const CharacterIndexPage(
          characters: [
            CharacterRow(
              globalId: 'Creature_Meatbug-WP_A',
              uniqueName: 'Creature_Meatbug',
              isDead: false,
              hasInventory: false,
              hasKnowledge: false,
              hasEvents: false,
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
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Meatbug'), findsOneWidget);
    expect(find.text('St Vlk Mud Sleeper'), findsOneWidget);
    expect(find.text('Creature_Meatbug-WP_A'), findsNothing);
    expect(find.text('ST_VLK_Mud_Sleeper'), findsNothing);
  });

  testWidgets('tapping an NPC selects it; tapping Player selects player', (
    tester,
  ) async {
    final selections = <Actor>[];
    await tester.pumpWidget(
      _pump(
        load: () async =>
            CharacterIndexPage(characters: [_actor('Lizard-WP_A')], total: 1),
        onSelect: selections.add,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Lizard'));
    await tester.pump();
    expect(selections.last.isPlayer, isFalse);
    expect(selections.last.id, 'Lizard-WP_A');
    expect(selections.last.name, 'Lizard');

    await tester.tap(find.text('Player'));
    await tester.pump();
    expect(selections.last.isPlayer, isTrue);
  });

  testWidgets('a dead actor shows the death avatar, an alive one the face', (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [
            _actor('Diego-WP_A', isDead: true),
            _actor('Lizard-WP_B'),
          ],
          total: 2,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Death is encoded in the LEADING avatar: exactly one skull (the dead
    // Diego) and exactly one live face (the alive Lizard).
    expect(find.byIcon(Icons.dangerous), findsOneWidget);
    expect(find.byIcon(Icons.face_outlined), findsOneWidget);
  });

  testWidgets('a badge-less row renders a short id subtitle fully', (
    tester,
  ) async {
    // No aspect flags → `_aspectBadges` returns null (no trailing widget), so a
    // short id renders WITHOUT the overflow ellipsis.
    const shortId = 'Diego-1';
    await tester.pumpWidget(
      _pump(
        load: () async =>
            CharacterIndexPage(characters: [_actor(shortId)], total: 1),
      ),
    );
    await tester.pumpAndSettle();

    final subtitle = tester.widget<Text>(find.text(shortId));
    expect(subtitle.data, shortId);
    final paragraph =
        find
                .descendant(
                  of: find.text(shortId),
                  matching: find.byType(RichText),
                )
                .evaluate()
                .single
                .renderObject!
            as RenderParagraph;
    expect(paragraph.didExceedMaxLines, isFalse);
  });

  testWidgets('a badge-less GlobalId subtitle spans most of the tile width', (
    tester,
  ) async {
    // Without aspect badges nothing reserves trailing width, so a long id is
    // laid out across (nearly) the whole tile before the ellipsis kicks in.
    const longId = 'Herek-WP_OM_TUNNEL_ME_01';
    await tester.pumpWidget(
      _pump(
        load: () async =>
            CharacterIndexPage(characters: [_actor(longId)], total: 1),
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
    expect(subtitleWidth, greaterThan(tileWidth * 0.7));
  });

  testWidgets('Player tile renders above the NPC search field', (tester) async {
    await tester.pumpWidget(
      _pump(
        load: () async =>
            CharacterIndexPage(characters: [_actor('Lizard-WP_A')], total: 1),
      ),
    );
    await tester.pumpAndSettle();

    final playerTop = tester.getTopLeft(find.text('Player')).dy;
    final searchTop = tester
        .getTopLeft(find.widgetWithText(TextField, 'Search NPCs'))
        .dy;
    expect(playerTop, lessThan(searchTop));
  });

  testWidgets('an actor resolves a localized name and keeps the id subtitle', (
    tester,
  ) async {
    // Catalog keyed by the character key (prefix before '-', lowercased).
    final catalog = <String, Map<String, String>>{
      'herek': {'english': 'Herek'},
    };
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [_actor('Herek-WP_OM_TUNNEL_ME_01')],
          total: 1,
        ),
        locCatalog: catalog,
      ),
    );
    await tester.pumpAndSettle();

    // Resolved localized name as the title.
    expect(find.text('Herek'), findsOneWidget);
    // Raw GlobalId retained as the subtitle.
    expect(find.text('Herek-WP_OM_TUNNEL_ME_01'), findsOneWidget);
  });

  testWidgets('an unresolved id falls back to a prettified name', (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [_actor('Creature_Meatbug-WP_OM_TUNNEL_01')],
          total: 1,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 'Creature_Meatbug' → prefix stripped + humanized → 'Meatbug'.
    expect(find.text('Meatbug'), findsOneWidget);
    expect(find.text('Creature_Meatbug-WP_OM_TUNNEL_01'), findsOneWidget);
  });

  testWidgets('searching a resolved NAME filters to that actor', (
    tester,
  ) async {
    // The full list is fetched ONCE. The fake always returns the full set, so
    // any filtering happening here is the CLIENT-SIDE id|name predicate.
    var calls = 0;
    // The first id's searchable key 'OC_NSC_01' does NOT contain 'Diego' —
    // only the RESOLVED loc name does.
    final catalog = <String, Map<String, String>>{
      'oc_nsc_01': {'english': 'Diego'},
    };
    await tester.pumpWidget(
      _pump(
        load: () async {
          calls++;
          return CharacterIndexPage(
            characters: [_actor('OC_NSC_01-WP_A'), _actor('Lizard-WP_B')],
            total: 2,
          );
        },
        locCatalog: catalog,
      ),
    );
    await tester.pumpAndSettle();

    // NPC titles live inside ListTile — scope the match so the search field's
    // own text can never match.
    Finder tileText(String s) =>
        find.descendant(of: find.byType(ListTile), matching: find.text(s));

    expect(tileText('Diego'), findsOneWidget);
    expect(tileText('Lizard'), findsOneWidget);

    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'Diego',
    );
    await tester.pumpAndSettle();

    expect(tileText('Diego'), findsOneWidget);
    expect(tileText('Lizard'), findsNothing);
    // The full list was fetched ONCE — searching does not re-hit the loader.
    expect(calls, 1);
  });

  // ---------------------------------------------------------------------------
  // The search query filters the ORPHAN group too (same id|name predicate as
  // the actor rows). When no orphan matches, the whole "Other" section —
  // header included — is hidden, mirroring the non-empty-only rule.
  // ---------------------------------------------------------------------------

  /// Hero actor + one NPC + one knowledge-only orphan — the standard fixture.
  CharacterIndexPage heroNpcOrphanPage() => CharacterIndexPage(
    characters: [
      _actor('Hero', uniqueName: 'Hero', hasKnowledge: true, hasEvents: true),
      _actor('Lizard-WP_A'),
      const CharacterRow(
        globalId: null,
        uniqueName: 'ST_VLK_Mud_Sleeper',
        isDead: false,
        hasInventory: false,
        hasKnowledge: true,
        hasEvents: false,
      ),
    ],
    total: 3,
  );

  testWidgets('a query matching only an NPC hides the orphan group entirely', (
    tester,
  ) async {
    await tester.pumpWidget(_pump(load: () async => heroNpcOrphanPage()));
    await tester.pumpAndSettle();

    // Row titles live inside ListTile — scope the match so the search field's
    // own text can never match.
    Finder tileText(String s) =>
        find.descendant(of: find.byType(ListTile), matching: find.text(s));

    // Unfiltered: the NPC, the orphan row, and the 'Other' header all render.
    expect(tileText('Lizard'), findsOneWidget);
    expect(tileText('St Vlk Mud Sleeper'), findsOneWidget);
    expect(find.text('Other'), findsOneWidget);

    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'Lizard',
    );
    await tester.pumpAndSettle();

    // Only the matching NPC remains; the orphan row AND the group header are
    // gone (not an always-rendered trailing section).
    expect(tileText('Lizard'), findsOneWidget);
    expect(tileText('St Vlk Mud Sleeper'), findsNothing);
    expect(find.text('Other'), findsNothing);
  });

  testWidgets('a query matching an orphan keeps it and filters the actors', (
    tester,
  ) async {
    await tester.pumpWidget(_pump(load: () async => heroNpcOrphanPage()));
    await tester.pumpAndSettle();

    // 'sleeper' matches only the orphan (id 'ST_VLK_Mud_Sleeper' and its
    // prettified name 'St Vlk Mud Sleeper' — same predicate as actors).
    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'sleeper',
    );
    await tester.pumpAndSettle();

    expect(find.text('St Vlk Mud Sleeper'), findsOneWidget);
    expect(find.text('Other'), findsOneWidget);
    // The non-matching actor is filtered out.
    expect(find.text('Lizard'), findsNothing);
  });

  testWidgets('searching an id substring still filters', (tester) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [_actor('Lizard-WP_A'), _actor('Herek-WP_B')],
          total: 2,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 'lizard' is a substring of the raw id 'Lizard-WP_A' (case-insensitive).
    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'lizard',
    );
    await tester.pumpAndSettle();

    expect(find.text('Lizard'), findsOneWidget);
    expect(find.text('Herek'), findsNothing);
  });

  testWidgets('pagination reflects the FILTERED count', (tester) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [for (var i = 0; i < 250; i++) _actor('Filler_$i-WP')],
          total: 250,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Unfiltered: 250 actor rows → "1–100 of 250" (page size 100).
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

  testWidgets('changing reloadKey reloads the list and resets state', (
    tester,
  ) async {
    var calls = 0;
    // A SINGLE stable loader (its identity never changes across rebuilds, like
    // the real `loadAllCharacters` method reference) that returns whatever the
    // current `nextRows` is — so only a reloadKey change can drive the reload.
    var nextRows = <CharacterRow>[_actor('Lizard-WP_A')];
    Future<CharacterIndexPage> stableLoad() async {
      calls++;
      return CharacterIndexPage(characters: nextRows, total: nextRows.length);
    }

    // Save A: shows Lizard.
    await tester.pumpWidget(_pump(load: stableLoad, reloadKey: 'save-A'));
    await tester.pumpAndSettle();
    expect(calls, 1);
    expect(find.text('Lizard'), findsOneWidget);

    // Switch to save B (new reloadKey only): the same loader re-runs and the
    // list swaps to the new save's characters.
    nextRows = <CharacterRow>[_actor('Herek-WP_B')];
    await tester.pumpWidget(_pump(load: stableLoad, reloadKey: 'save-B'));
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('Herek'), findsOneWidget);
    // The previous save's actor is gone (state reset, not appended).
    expect(find.text('Lizard'), findsNothing);
  });

  testWidgets('actor rows are sorted alphabetically by resolved name', (
    tester,
  ) async {
    // Fed out of order; prettified names are Zeta / Alpha / Mango.
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [_actor('Zeta-1'), _actor('Alpha-1'), _actor('Mango-1')],
          total: 3,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final alphaY = tester.getTopLeft(find.text('Alpha')).dy;
    final mangoY = tester.getTopLeft(find.text('Mango')).dy;
    final zetaY = tester.getTopLeft(find.text('Zeta')).dy;
    expect(alphaY, lessThan(mangoY));
    expect(mangoY, lessThan(zetaY));
  });
}
