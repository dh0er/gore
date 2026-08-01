import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/ui/location_picker_dialog.dart';
import 'package:goresave/l10n/app_localizations_de.dart';

import 'support/l10n_test_app.dart';

/// The shared location picker. It is deliberately write-agnostic — it returns a
/// [LocationPick] and nothing else — so both the player editor and the NPC
/// position panel can translate the same result into their own command.
void main() {
  /// `FP_NC_MISFILED_GUARD` is named for the New Camp but filed under `OC`:
  /// the catalog's area assignment comes from a spatial pass and is not
  /// authoritative, so a search must be able to reach it anyway.
  LocationCatalog buildCatalog() => LocationCatalog.fromJsonString(
    '{"version":1,'
    '"areas":['
    '{"id":"OC","label":"Old Camp","locId":"area_oldcamp_notification"},'
    '{"id":"NC","label":"New Camp","locId":null}],'
    '"spots":['
    '{"n":"FP_OC_STAND_YARD_1","x":110520.3,"y":-102715.1,"z":-3719.6,'
    '"w":111.4,"a":"OC"},'
    '{"n":"FP_OC_STAND_YARD_2","x":1.5,"y":-2.5,"z":3.5,"w":90.0,"a":"OC"},'
    '{"n":"FP_NC_MISFILED_GUARD","x":7.25,"y":8.5,"z":9.75,"w":12.0,'
    '"a":"OC"},'
    '{"n":"IO_NC_ANVIL_1","x":10.0,"y":20.0,"z":30.0,"w":-45.5,"a":"NC"},'
    '{"n":"WP_UNASSIGNED_SPOT","x":4.0,"y":5.0,"z":6.0,"w":0.0,"a":""}]}',
  );

  /// Pump a host with an "open" button and return the picker's result holder.
  Future<List<LocationPick?>> pumpPicker(
    WidgetTester tester,
    LocationCatalog catalog, {
    Locale locale = const Locale('en'),
  }) async {
    final picked = <LocationPick?>[];
    await tester.pumpWidget(
      wrapWithL10n(
        locale: locale,
        Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async => picked.add(
                await showLocationPickerDialog(
                  context,
                  catalogOverride: catalog,
                ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    return picked;
  }

  /// Type into the search field and let the debounce elapse.
  Future<void> search(WidgetTester tester, String query) async {
    await tester.enterText(
      find.widgetWithText(TextField, 'Search entries'),
      query,
    );
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
  }

  testWidgets('searching and tapping a tile returns that spot', (tester) async {
    final picked = await pumpPicker(tester, buildCatalog());

    expect(find.text('Choose a location'), findsOneWidget);
    await search(tester, 'misfiled');

    expect(find.text('FP_NC_MISFILED_GUARD'), findsOneWidget);
    expect(find.text('FP_OC_STAND_YARD_1'), findsNothing);

    await tester.tap(find.text('FP_NC_MISFILED_GUARD'));
    await tester.pumpAndSettle();

    expect(picked, hasLength(1));
    final pick = picked.single!;
    expect(pick.spot.name, 'FP_NC_MISFILED_GUARD');
    expect(pick.spot.x, 7.25);
    expect(pick.spot.y, 8.5);
    expect(pick.spot.z, 9.75);
    expect(pick.spot.yaw, 12.0);
  });

  testWidgets('the rotation checkbox is off by default and opt-in', (
    tester,
  ) async {
    var picked = await pumpPicker(tester, buildCatalog());

    // A spot's yaw is the heading of someone USING it, not a sensible facing
    // for a relocated character — so the box starts cleared.
    expect(
      tester.widget<CheckboxListTile>(find.byType(CheckboxListTile)).value,
      isFalse,
    );

    await search(tester, 'anvil');
    await tester.tap(find.text('IO_NC_ANVIL_1'));
    await tester.pumpAndSettle();
    expect(picked.single!.applyRotation, isFalse);

    // Ticking it flips the flag the callers act on.
    picked = await pumpPicker(tester, buildCatalog());
    await tester.tap(find.text("Also apply the spot's orientation"));
    await tester.pumpAndSettle();
    expect(
      tester.widget<CheckboxListTile>(find.byType(CheckboxListTile)).value,
      isTrue,
    );
    await search(tester, 'anvil');
    await tester.tap(find.text('IO_NC_ANVIL_1'));
    await tester.pumpAndSettle();
    expect(picked.single!.applyRotation, isTrue);
    expect(picked.single!.spot.yaw, -45.5);
  });

  testWidgets('search spans the whole catalog regardless of the area', (
    tester,
  ) async {
    await pumpPicker(tester, buildCatalog());

    // Narrow to New Camp: only its single spot is listed.
    await tester.tap(find.text('New Camp (1)'));
    await tester.pumpAndSettle();
    expect(find.text('IO_NC_ANVIL_1'), findsOneWidget);
    expect(find.text('FP_NC_MISFILED_GUARD'), findsNothing);

    // A query then searches EVERYTHING: the spot mis-filed under Old Camp by
    // the catalog's spatial pass is still reachable by name.
    await search(tester, 'fp_nc');
    expect(find.text('FP_NC_MISFILED_GUARD'), findsOneWidget);
  });

  testWidgets('areas are ordered by size with the unassigned bucket last', (
    tester,
  ) async {
    await pumpPicker(tester, buildCatalog());

    final labels = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byType(SingleChildScrollView),
            matching: find.byType(Text),
          ),
        )
        .map((t) => t.data)
        .whereType<String>()
        .toList();
    expect(labels, ['All (5)', 'Old Camp (3)', 'New Camp (1)', 'Other (1)']);
  });

  testWidgets('an area the game does not name is still shown in German', (
    tester,
  ) async {
    // `OG` carries no loc id — before this the sidebar showed "Orc Graveyard"
    // next to half a dozen German names. The label now comes from the app's own
    // ARB, so the dialog has to reach it without any loc catalog loaded, which
    // is exactly the situation in a test.
    await pumpPicker(
      tester,
      LocationCatalog.fromJsonString(
        '{"version":1,'
        '"areas":[{"id":"OG","label":"Orc Graveyard","locId":null}],'
        '"spots":[{"n":"FP_OG_TOMB_1","x":1.0,"y":2.0,"z":3.0,"w":0.0,'
        '"a":"OG"}]}',
      ),
      locale: const Locale('de'),
    );

    expect(find.text('Orkfriedhof (1)'), findsOneWidget);
    expect(find.text('Orc Graveyard (1)'), findsNothing);
  });

  /// The regression this whole feature is: nine areas had no loc id and fell
  /// through to their generated English label, so the sidebar mixed languages.
  /// Data-driven over the shipped asset on purpose — an area added by a future
  /// `gore location-catalog` run without a translation fails HERE, not in a
  /// screenshot somebody happens to look at.
  test('no area of the bundled catalog is left untranslated', () {
    final catalog = LocationCatalog.fromJsonString(
      File('assets/location_catalog.json').readAsStringSync(),
    );
    expect(
      catalog.areas.length,
      greaterThan(20),
      reason: 'this must run against the shipped asset, not a fixture',
    );

    // German spells a few of these exactly as English does, and a rule that
    // forbids the correct word is worse than no rule — so the exception is one
    // named id, not a loosened assertion.
    const sameWordInGerman = {'HC'}; // die Tundra
    final german = AppLocalizationsDe();

    for (final area in catalog.areas) {
      final gameNamesIt = area.locId != null && area.locId!.isNotEmpty;
      final ours = appAreaLabel(area.id, german);
      expect(
        gameNamesIt || ours != null,
        isTrue,
        reason:
            'area ${area.id} ("${area.label}") has neither a loc id into the '
            "game's own strings nor an entry in appAreaLabel, so it would "
            'render English in every language',
      );
      if (ours == null) continue;
      expect(
        ours.trim(),
        isNotEmpty,
        reason: 'area ${area.id} resolves to an empty German label',
      );
      if (!sameWordInGerman.contains(area.id)) {
        expect(
          ours,
          isNot(area.label),
          reason:
              'area ${area.id} still shows the raw English label in German',
        );
      }
    }
  });

  testWidgets('an empty catalog reports that it could not be loaded', (
    tester,
  ) async {
    await pumpPicker(
      tester,
      LocationCatalog.fromJsonString('{"areas":[],"spots":[]}'),
    );

    expect(
      find.text('The location catalog could not be loaded.'),
      findsOneWidget,
    );
  });
}
