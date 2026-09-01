import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/npc_position_fake_core.dart';

/// "Auf Spawn zurücksetzen" is a pure-UI action: it fills the six editable
/// fields from the loaded spawn reference and lets the normal recompute decide
/// what to queue. It must be DISABLED when there is nothing sensible to reset
/// to — no spawn location, a (0,0,0) spawn (the NPC was never placed), or a
/// pose that already equals the spawn.
void main() {
  const resetLabel = 'Reset to spawn position';

  Finder resetButton() => find.widgetWithText(FilledButton, resetLabel);

  bool resetEnabled(WidgetTester tester) =>
      tester.widget<FilledButton>(resetButton()).onPressed != null;

  /// Every fixture id here prettifies to the same display name, so the list
  /// folds them into one expandable row — open it before reaching for an id.
  Future<void> select(WidgetTester tester, String id) async {
    if (find.text(id).evaluate().isEmpty) {
      await tester.tap(find.textContaining(RegExp(r'^Npc \(\d+\)$')));
      await tester.pumpAndSettle();
    }
    await tester.tap(find.text(id));
    await tester.pumpAndSettle();
  }

  final core = NpcPositionCoreService({
    // Moved away from its spawn: reset is meaningful.
    'Npc-Moved': const FakePose(
      location: (1.0, 2.0, 3.0),
      rotation: (10.0, 20.0, 30.0),
      spawnLocation: (100.0, 200.0, 300.0),
      spawnRotation: (0.0, 90.0, 0.0),
    ),
    // No spawn member at all.
    'Npc-NoSpawn': const FakePose(location: (1.0, 2.0, 3.0)),
    // Never placed: the spawn reference is the origin and means nothing.
    'Npc-ZeroSpawn': const FakePose(
      location: (1.0, 2.0, 3.0),
      spawnLocation: (0.0, 0.0, 0.0),
    ),
    // Already standing on its spawn.
    'Npc-AtSpawn': const FakePose(
      location: (5.0, 6.0, 7.0),
      rotation: (0.0, 90.0, 0.0),
      spawnLocation: (5.0, 6.0, 7.0),
      spawnRotation: (0.0, 90.0, 0.0),
    ),
  });

  testWidgets('reset fills the six fields from spawn and queues the edit', (
    tester,
  ) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await select(tester, 'Npc-Moved');

    // The spawn pose is shown read-only, as text — not as disabled fields.
    expect(find.text('Spawn position (reference)'), findsOneWidget);
    expect(find.text('Location X: 100'), findsOneWidget);
    expect(find.text('Rotation yaw: 90'), findsOneWidget);

    expect(resetEnabled(tester), isTrue);
    await tester.tap(resetButton());
    await tester.pumpAndSettle();

    // All six editable fields now hold the spawn values …
    expect(positionFieldText(tester, 'location:x'), '100');
    expect(positionFieldText(tester, 'location:y'), '200');
    expect(positionFieldText(tester, 'location:z'), '300');
    expect(positionFieldText(tester, 'rotation:pitch'), '0');
    expect(positionFieldText(tester, 'rotation:yaw'), '90');
    expect(positionFieldText(tester, 'rotation:roll'), '0');
    // … and both groups differ from the saved pose, so both are queued.
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);
    // Nothing left to reset to.
    expect(resetEnabled(tester), isFalse);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    Map<String, Object?> valueFor(String leaf) =>
        edits.map((e) => e['value'] as Map<String, Object?>).firstWhere(
          (v) => (v['path'] as List).last == leaf,
        );
    expect(valueFor('CharacterLocation')['value'], {
      'x': 100.0,
      'y': 200.0,
      'z': 300.0,
    });
    expect(valueFor('CharacterRotation')['value'], {
      'pitch': 0.0,
      'yaw': 90.0,
      'roll': 0.0,
    });
  });

  testWidgets('reset is disabled without a spawn location', (tester) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await select(tester, 'Npc-NoSpawn');

    expect(resetButton(), findsOneWidget);
    expect(resetEnabled(tester), isFalse);
    // No spawn members at all: nothing is listed under the reference heading.
    expect(find.textContaining('Location X: '), findsNothing);
  });

  testWidgets('reset is disabled for a (0,0,0) spawn', (tester) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await select(tester, 'Npc-ZeroSpawn');

    expect(resetEnabled(tester), isFalse);
  });

  testWidgets('reset is disabled when the pose already equals the spawn', (
    tester,
  ) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await select(tester, 'Npc-AtSpawn');

    expect(resetEnabled(tester), isFalse);

    // Move away and it becomes available again.
    await tester.enterText(positionField('location:x'), '42');
    await tester.pump();
    expect(resetEnabled(tester), isTrue);
  });

  testWidgets('a never-placed NPC is flagged but stays editable', (
    tester,
  ) async {
    final zeroCore = NpcPositionCoreService({
      'Npc-Unplaced': const FakePose(
        location: (0.0, 0.0, 0.0),
        spawnLocation: (0.0, 0.0, 0.0),
      ),
    });
    await pumpPositionApp(tester, zeroCore);
    await openPositionTab(tester);
    await select(tester, 'Npc-Unplaced');

    expect(
      find.textContaining('never been placed in the world'),
      findsOneWidget,
    );
    // The warning does not lock the fields.
    await tester.enterText(positionField('location:x'), '5');
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
  });
}
