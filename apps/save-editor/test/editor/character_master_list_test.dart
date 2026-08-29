import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/glossary_portrait.dart';
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

  testWidgets('renders Player and the actors, never one that never spawned', (
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
    // The knowledge-only character has no actor in the world, so it is not
    // listed at all — neither a row nor the group header that used to carry it.
    expect(find.text('Weitere'), findsNothing);
    expect(find.text('St Vlk Mud Sleeper'), findsNothing);

    // Tap the actor row (found by its resolved, prettified display name).
    await tester.tap(find.text(laresDisplay));
    await tester.pumpAndSettle();
    expect(picked?.id, 'NC_ORG_Lares_801-WP_X');
    expect(picked?.uniqueName, 'NC_ORG_Lares_801');
    expect(picked?.isOrphan, isFalse);
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

      // Pinned Player + the normal NPC. The knowledge-only character never
      // spawned, so it is absent along with the group that used to hold it.
      expect(find.text('Player'), findsOneWidget);
      expect(find.text('Lizard'), findsOneWidget);
      expect(find.text('St Vlk Mud Sleeper'), findsNothing);
      expect(find.text('Other'), findsNothing);
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

  testWidgets('hides actor ids while keeping readable names', (tester) async {
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
    expect(find.text('Creature_Meatbug-WP_A'), findsNothing);
    // The knowledge-only character is out of the list entirely.
    expect(find.text('St Vlk Mud Sleeper'), findsNothing);
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

  testWidgets('a killed actor keeps its picture and gains a death badge', (
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

    // Death says something ABOUT a character, it is not who the character is —
    // so both rows ask for their own picture and the killed one carries the
    // game's death mark among its trailing badges instead.
    GlossaryPortrait markOf(String name) => tester.widget<GlossaryPortrait>(
      find.descendant(
        of: find.ancestor(of: find.text(name), matching: find.byType(ListTile)),
        matching: find.byType(GlossaryPortrait),
      ),
    );
    expect(markOf('Diego').npcUniqueName, startsWith('Diego'));
    expect(markOf('Diego').fallbackGameIcon, isNull);
    expect(markOf('Lizard').npcUniqueName, startsWith('Lizard'));

    Finder badge(String name, String icon) => find.descendant(
      of: find.ancestor(of: find.text(name), matching: find.byType(ListTile)),
      matching: find.byWidgetPredicate(
        (widget) => widget is GameIcon && widget.name == icon,
      ),
    );
    expect(badge('Diego', gameIconDead), findsOneWidget);
    expect(badge('Lizard', gameIconDead), findsNothing);

    // Every mark reserves the same box, so the names line up down the list.
    final widths = tester
        .widgetList<GlossaryPortrait>(find.byType(GlossaryPortrait))
        .map((mark) => mark.width)
        .toSet();
    expect(widths, hasLength(1));
  });

  testWidgets("the badges are the game's own glyphs, events is not one", (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        load: () async => CharacterIndexPage(
          characters: [
            const CharacterRow(
              globalId: 'Diego-WP_A',
              uniqueName: 'Diego',
              isDead: false,
              hasInventory: true,
              hasKnowledge: true,
              hasEvents: true,
              isTrader: true,
            ),
          ],
          total: 1,
        ),
      ),
    );
    await tester.pumpAndSettle();

    Finder glyph(String name) => find.byWidgetPredicate(
      (widget) => widget is GameIcon && widget.name == name,
    );
    expect(glyph(gameIconKnowledge), findsOneWidget);
    expect(glyph(gameIconTrade), findsOneWidget);
    // Every actor records events, so the badge said nothing and is gone.
    expect(find.byIcon(Icons.history), findsNothing);
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

  /// Hero actor + one NPC + one knowledge-only character — the standard
  /// fixture.
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

  testWidgets('a character that never spawned is absent, query or not', (
    tester,
  ) async {
    await tester.pumpWidget(_pump(load: () async => heroNpcOrphanPage()));
    await tester.pumpAndSettle();

    // Row titles live inside ListTile — scope the match so the search field's
    // own text can never match.
    Finder tileText(String s) =>
        find.descendant(of: find.byType(ListTile), matching: find.text(s));

    expect(tileText('Lizard'), findsOneWidget);
    expect(tileText('St Vlk Mud Sleeper'), findsNothing);
    expect(find.text('Other'), findsNothing);
    // It is out of the COUNT too, not merely hidden.
    expect(find.text('1–1 of 1'), findsOneWidget);

    // Searching for it by name finds nothing to show.
    await tester.enterText(
      find.widgetWithText(TextField, 'Search NPCs'),
      'sleeper',
    );
    await tester.pumpAndSettle();
    expect(tileText('St Vlk Mud Sleeper'), findsNothing);
    expect(tileText('Lizard'), findsNothing);
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
          // Distinct display names: same-named actors would fold into one
          // group and page as a single line (covered separately below).
          characters: [for (var i = 0; i < 250; i++) _actor('Filler_${i}x-WP')],
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
      'Filler_42x-',
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

  // ---------------------------------------------------------------------------
  // Same-named actors — the forty scavengers, the fifty guards — fold into one
  // expandable row so the list stays readable.
  // ---------------------------------------------------------------------------

  testWidgets('same-named actors fold into one row carrying their count', (
    tester,
  ) async {
    await tester.pumpWidget(
      _pump(
        showObjectIds: false,
        load: () async => CharacterIndexPage(
          characters: [
            _actor('Bloodfly-WP_A'),
            _actor('Bloodfly-WP_B'),
            _actor('Bloodfly-WP_C'),
            _actor('Herek-WP_D'),
          ],
          total: 4,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // One line for the three, one for the lone actor.
    expect(find.text('Bloodfly (3)'), findsOneWidget);
    expect(find.text('Bloodfly'), findsNothing);
    expect(find.text('Herek'), findsOneWidget);
    // Paging counts the LINES, so the number matches what is on screen.
    expect(find.text('1–2 of 2'), findsOneWidget);

    // Opening it reveals every member; the count line stays put.
    await tester.tap(find.text('Bloodfly (3)'));
    await tester.pumpAndSettle();
    expect(find.text('Bloodfly'), findsNWidgets(3));
    expect(find.text('Bloodfly (3)'), findsOneWidget);
    expect(find.text('1–2 of 2'), findsOneWidget);

    // And closing it puts the members away again.
    await tester.tap(find.text('Bloodfly (3)'));
    await tester.pumpAndSettle();
    expect(find.text('Bloodfly'), findsNothing);
  });

  testWidgets('a member row selects the actor behind it, not the group', (
    tester,
  ) async {
    final selections = <Actor>[];
    await tester.pumpWidget(
      _pump(
        showObjectIds: false,
        onSelect: selections.add,
        load: () async => CharacterIndexPage(
          characters: [
            _actor('Bloodfly-WP_A'),
            _actor('Bloodfly-WP_B'),
            _actor('Bloodfly-WP_C'),
          ],
          total: 3,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Bloodfly (3)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Bloodfly').first);
    await tester.pump();
    expect(selections.single.id, 'Bloodfly-WP_A');
  });

  testWidgets('two of a name stay two rows; a third folds them', (
    tester,
  ) async {
    // Hiding a pair behind a click buys nothing.
    CharacterIndexPage page(int count) => CharacterIndexPage(
      characters: [for (var i = 0; i < count; i++) _actor('Bloodfly-WP_$i')],
      total: count,
    );
    await tester.pumpWidget(
      _pump(showObjectIds: false, load: () async => page(2)),
    );
    await tester.pumpAndSettle();
    expect(find.text('Bloodfly'), findsNWidgets(2));
    expect(find.text('Bloodfly (2)'), findsNothing);

    await tester.pumpWidget(
      _pump(showObjectIds: false, reloadKey: 'k2', load: () async => page(3)),
    );
    await tester.pumpAndSettle();
    expect(find.text('Bloodfly'), findsNothing);
    expect(find.text('Bloodfly (3)'), findsOneWidget);
  });

  testWidgets('an opened group may carry the page past its own size', (
    tester,
  ) async {
    // 100 lone actors fill the page exactly; opening a group on it adds its
    // members on top of the hundred.
    await tester.pumpWidget(
      _pump(
        showObjectIds: false,
        load: () async => CharacterIndexPage(
          characters: [
            for (var i = 0; i < 99; i++) _actor('Filler_${i}x-WP'),
            _actor('Aaa-WP_A'),
            _actor('Aaa-WP_B'),
            _actor('Aaa-WP_C'),
          ],
          total: 102,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 99 lone actors + one group = 100 lines, one full page.
    expect(find.text('1–100 of 100'), findsOneWidget);
    await tester.tap(find.text('Aaa (3)'));
    await tester.pumpAndSettle();
    // The three members are on the page now; the paging numbers do not move.
    expect(find.text('1–100 of 100'), findsOneWidget);
    expect(find.text('Aaa'), findsNWidgets(3));
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
