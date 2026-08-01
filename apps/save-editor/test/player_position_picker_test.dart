import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart'
    show formatHeroValue;
import 'package:goresave/providers/data_providers.dart';

import 'support/npc_position_fake_core.dart';

/// The player reaches the SAME picker dialog as an NPC, but keeps its own write:
/// the picker only seeds controllers, and the existing `_updatePending()` turns
/// them into the one `private.player.setTransform` edit under the `'transform'`
/// key. One dialog, two commands.
void main() {
  /// A spot the bundled catalog is already pinned on by catalog_loaders_test.
  /// Its name is unique as a substring, so a search resolves to one row.
  const spotName = 'IO_SC_ANVIL_2';

  NpcPositionCoreService buildCore() => NpcPositionCoreService(
    {'Lizard-A': const FakePose()},
    playerTransform: ((1.0, 2.0, 3.0), (4.0, 5.0, 6.0)),
  );

  LocationSpot spotOf(LocationCatalog catalog) =>
      catalog.spots.firstWhere((s) => s.name == spotName);

  String fieldText(WidgetTester tester, String label) => tester
      .widget<EditableText>(
        find.descendant(
          of: find.widgetWithText(TextField, label),
          matching: find.byType(EditableText),
        ),
      )
      .controller
      .text;

  testWidgets(
    'picking a spot fills the player location fields and queues a '
    'setTransform edit',
    (tester) async {
      final core = buildCore();
      final spot = spotOf(await loadBundledCatalog(tester));
      await pumpPositionApp(tester, core);
      await openPositionTab(tester);

      // The Player row is pinned and selected by default, so the Position tab
      // shows the player's transform editor.
      expect(find.widgetWithText(TextField, 'Location X'), findsOneWidget);
      expect(fieldText(tester, 'Location X'), '1');

      await pickLocationSpot(tester, spotName);

      expect(fieldText(tester, 'Location X'), formatHeroValue(spot.x));
      expect(fieldText(tester, 'Location Y'), formatHeroValue(spot.y));
      expect(fieldText(tester, 'Location Z'), formatHeroValue(spot.z));
      // Rotation was not opted into, so the saved pose stays untouched.
      expect(fieldText(tester, 'Rotation pitch'), '4');
      expect(fieldText(tester, 'Rotation yaw'), '5');
      expect(fieldText(tester, 'Rotation roll'), '6');

      // The picker went through the EXISTING pending path: the single
      // `'transform'` key, not a second write route.
      expect(
        positionContainer(tester).read(editorProvider).pendingEdits.keys,
        contains('transform'),
      );
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(1));
      expect(edits.single['path'], 'private.player.setTransform');
      expect(edits.single['value'], {
        'location': {'x': spot.x, 'y': spot.y, 'z': spot.z},
        'rotation': {'pitch': 4.0, 'yaw': 5.0, 'roll': 6.0},
      });
    },
  );

  testWidgets('opting into the orientation writes yaw only, pitch and roll 0', (
    tester,
  ) async {
    final core = buildCore();
    final spot = spotOf(await loadBundledCatalog(tester));
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);

    await pickLocationSpot(tester, spotName, applyRotation: true);

    // The catalog stores yaw alone — pitch and roll are zeroed rather than
    // invented, because a spot's pitch would visibly tilt a standing pawn.
    expect(fieldText(tester, 'Rotation pitch'), '0');
    expect(fieldText(tester, 'Rotation yaw'), formatHeroValue(spot.yaw));
    expect(fieldText(tester, 'Rotation roll'), '0');

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits.single['value'], {
      'location': {'x': spot.x, 'y': spot.y, 'z': spot.z},
      'rotation': {'pitch': 0.0, 'yaw': spot.yaw, 'roll': 0.0},
    });
  });
}
