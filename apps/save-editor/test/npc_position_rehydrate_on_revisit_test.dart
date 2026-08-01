import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/npc_position_fake_core.dart';

/// Regression test for Bug #8 in its position form: switching away from an NPC
/// whose position draft is queued and returning must REHYDRATE the panel's
/// fields from the stored per-NPC entry. Editing a SECOND axis on the revisited
/// NPC must keep BOTH values — without rehydration the panel would re-seed from
/// disk and the recompute would emit a triplet carrying the SAVED x, silently
/// dropping the earlier edit (while the Save badge had counted it).
/// 1:1 with npc_attribute_rehydrate_on_revisit_test.dart.
void main() {
  testWidgets(
    'returning to an edited NPC keeps the earlier axis when editing again',
    (tester) async {
      final core = NpcPositionCoreService({
        'Lizard-A': const FakePose(
          location: (10.0, 11.0, 12.0),
          rotation: (0.0, 0.0, 0.0),
          spawnLocation: (500.0, 600.0, 700.0),
        ),
        'Lizard-B': const FakePose(
          location: (20.0, 21.0, 22.0),
          rotation: (0.0, 0.0, 0.0),
          spawnLocation: (500.0, 600.0, 700.0),
        ),
      });
      await pumpPositionApp(tester, core);
      await openPositionTab(tester);

      // Edit NPC-A's X.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      expect(positionField('location:x'), findsOneWidget);
      await tester.enterText(positionField('location:x'), '111');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to NPC-B, then back to NPC-A. The panel reload drops its local
      // drafts but A's queued edit survives in the registry.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();

      // X shows the rehydrated draft (111), not the saved 10.
      expect(positionFieldText(tester, 'location:x'), '111');
      // The untouched axes still show A's saved values.
      expect(positionFieldText(tester, 'location:y'), '11');
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Edit a SECOND axis. Both must survive into the write.
      await tester.enterText(positionField('location:y'), '99');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      // One struct write for the location leaf, carrying BOTH edited axes.
      expect(edits, hasLength(1));
      final value = edits.single['value'] as Map<String, Object?>;
      expect((value['path'] as List).cast<String>(), contains('{Lizard-A}'));
      expect((value['path'] as List).last, 'CharacterLocation');
      expect(value['value'], {'x': 111.0, 'y': 99.0, 'z': 12.0});
    },
  );

  testWidgets('a rehydrated rotation draft survives alongside a location edit', (
    tester,
  ) async {
    final core = NpcPositionCoreService({
      'Lizard-A': const FakePose(
        location: (10.0, 11.0, 12.0),
        rotation: (1.0, 2.0, 3.0),
        spawnLocation: (500.0, 600.0, 700.0),
      ),
      'Lizard-B': const FakePose(location: (20.0, 21.0, 22.0)),
    });
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);

    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
    await tester.enterText(positionField('rotation:yaw'), '180');
    await tester.pump();

    // Leave and come back: the rotation draft must be restored.
    await tester.tap(find.text('Lizard-B'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
    expect(positionFieldText(tester, 'rotation:yaw'), '180');

    // Now edit a LOCATION axis: the two groups are separate struct writes, and
    // the rotation one must not be dropped when the location one is emitted.
    await tester.enterText(positionField('location:z'), '77');
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(2));
    Map<String, Object?> valueFor(String leaf) =>
        edits.map((e) => e['value'] as Map<String, Object?>).firstWhere(
          (v) => (v['path'] as List).last == leaf,
        );
    expect(valueFor('CharacterLocation')['value'], {
      'x': 10.0,
      'y': 11.0,
      'z': 77.0,
    });
    expect(valueFor('CharacterRotation')['value'], {
      'pitch': 1.0,
      'yaw': 180.0,
      'roll': 3.0,
    });
  });
}
