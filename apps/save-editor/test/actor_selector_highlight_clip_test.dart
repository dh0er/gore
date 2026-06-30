import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/ui/actor_selector.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/ui/design/app_theme.dart';

import 'support/l10n_test_app.dart';

NpcActor _npc(String id) =>
    NpcActor(id: id, isDead: false, hp: 80, maxHp: 80);

/// Regression guard for the selected-NPC-tile teal highlight bleeding ABOVE the
/// scrollable list's top edge into the pagination/header region.
///
/// Root cause: `ListTile.selectedTileColor` is painted by the NEAREST enclosing
/// [Material]. With only a [ClipRect] (no Material) around the [ListView], the
/// selected fill was drawn onto the ancestor Scaffold Material — OUTSIDE the
/// clip — so a scrolled-up selected tile bled above the list top. The fix wraps
/// the list in its own [Material] inside the [ClipRect], so the highlight is
/// painted on (and clipped by) the list's bounds.
///
/// This is a STRUCTURAL test rather than a golden image: the repo uses no
/// `matchesGoldenFile`, and golden font rendering is machine/CI dependent.
void main() {
  Widget pump({required Actor selected, required List<NpcActor> npcs}) {
    Future<NpcActorsPage> fakeLoad({
      String query = '',
      int offset = 0,
      int limit = 100,
    }) async =>
        NpcActorsPage(npcs: npcs, total: npcs.length, offset: 0, limit: limit);

    return ProviderScope(
      child: MaterialApp(
        locale: const Locale('en'),
        theme: buildGoresaveTheme(),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 380,
              height: 480,
              child: ActorSelector(
                selected: selected,
                onSelect: (_) {},
                loadNpcs: fakeLoad,
                reloadKey: 'k',
                locCatalog: const {},
                lang: kGameLangs.first,
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets(
    'a Material wraps the NPC ListView INSIDE the ClipRect so the selected '
    'tile highlight is clipped to the list (no bleed above the top edge)',
    (tester) async {
      final npcs = <NpcActor>[
        for (var i = 0; i < 17; i++) _npc('Guard_$i-WP_A'),
      ];
      await tester.pumpWidget(
        pump(
          selected: Actor.npc(id: 'Guard_0-WP_A', name: 'Guard 0'),
          npcs: npcs,
        ),
      );
      await tester.pumpAndSettle();

      // The list is wrapped in a ClipRect that contains a Material which in turn
      // contains the ListView. This Material is what paints the selected-tile
      // fill, and being inside the ClipRect it is clipped to the list bounds.
      final clipRect = find.ancestor(
        of: find.byType(ListView),
        matching: find.byType(ClipRect),
      );
      expect(clipRect, findsWidgets);

      final materialInClip = find.descendant(
        of: clipRect.first,
        matching: find.byType(Material),
      );
      expect(
        materialInClip,
        findsWidgets,
        reason:
            'the ListView must have a Material ancestor inside the ClipRect '
            'so selectedTileColor is painted on a clipped layer',
      );
      // That Material is an ancestor of the ListView (it wraps it).
      expect(
        find.descendant(
          of: materialInClip.first,
          matching: find.byType(ListView),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'a scrolled-up selected tile never paints its highlight above the list top',
    (tester) async {
      final npcs = <NpcActor>[
        for (var i = 0; i < 17; i++) _npc('Guard_$i-WP_A'),
      ];
      await tester.pumpWidget(
        pump(
          selected: Actor.npc(id: 'Guard_0-WP_A', name: 'Guard 0'),
          npcs: npcs,
        ),
      );
      await tester.pumpAndSettle();

      // Scroll the selected first tile partway past the top edge.
      await tester.drag(find.byType(ListView), const Offset(0, -28));
      await tester.pumpAndSettle();

      // The Material that paints the highlight sits inside the ClipRect, whose
      // top is the list's top edge. The highlight therefore cannot be painted
      // above that boundary regardless of scroll offset. Assert the Material's
      // paint bounds do not extend above the ClipRect's top.
      final clipRect = find
          .ancestor(of: find.byType(ListView), matching: find.byType(ClipRect))
          .first;
      final clipTop = tester.getTopLeft(clipRect).dy;

      final material = find
          .descendant(of: clipRect, matching: find.byType(Material))
          .first;
      final materialTop = tester.getTopLeft(material).dy;

      // The list's Material starts at (or below) the clip's top edge — the
      // highlight it paints is bounded by the clip, so nothing bleeds upward.
      expect(materialTop, greaterThanOrEqualTo(clipTop - 0.5));
    },
  );
}
