import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/ui/position_detail.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/npc_position_fake_core.dart';

/// The guard against rebuilding NPC position editing.
///
/// It was built once, on the belief that the save's per-NPC pose is applied on
/// load. It is not: a UE4SS runtime probe rewrote `CharacterLocation`,
/// `SpawnLocation` and `DailyRoutineClass` for two NPCs (one streamed out, one
/// simulated), loaded the byte-verified save, and read back the ORIGINAL
/// pre-edit values in every field. The game restores an NPC's placement from
/// the level's WorldPointActor, not from the savegame.
///
/// So the panel shows the four triplets and offers NO way to change them. If a
/// future change re-introduces a field, a picker or a pending edit here, this
/// test fails — which is the point.
void main() {
  NpcPositionCoreService buildCore() => NpcPositionCoreService({
    'Lizard-A': const FakePose(
      location: (11.0, 12.0, 13.0),
      rotation: (14.0, 15.0, 16.0),
      spawnLocation: (21.0, 22.0, 23.0),
      spawnRotation: (24.0, 25.0, 26.0),
    ),
  });

  Future<void> openNpc(WidgetTester tester, NpcPositionCoreService core) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
  }

  testWidgets('the saved pose and the spawn reference are both shown', (
    tester,
  ) async {
    await openNpc(tester, buildCore());

    // CharacterLocation / CharacterRotation …
    expect(find.text('Location X: 11'), findsOneWidget);
    expect(find.text('Location Y: 12'), findsOneWidget);
    expect(find.text('Location Z: 13'), findsOneWidget);
    expect(find.text('Rotation pitch: 14'), findsOneWidget);
    expect(find.text('Rotation yaw: 15'), findsOneWidget);
    expect(find.text('Rotation roll: 16'), findsOneWidget);
    // … and SpawnLocation / SpawnRotation, under their own heading.
    expect(find.text('Spawn position (reference)'), findsOneWidget);
    expect(find.text('Location X: 21'), findsOneWidget);
    expect(find.text('Location Y: 22'), findsOneWidget);
    expect(find.text('Location Z: 23'), findsOneWidget);
    expect(find.text('Rotation pitch: 24'), findsOneWidget);
    expect(find.text('Rotation yaw: 25'), findsOneWidget);
    expect(find.text('Rotation roll: 26'), findsOneWidget);

    // The reason is stated, not left for the user to guess at.
    expect(
      find.textContaining('restores an NPC'),
      findsOneWidget,
      reason: 'the panel must say why the values cannot be changed',
    );
  });

  testWidgets('nothing in the panel can be edited, and nothing is queued', (
    tester,
  ) async {
    final core = buildCore();
    await openNpc(tester, core);

    final panel = find.byType(NpcPositionPanel);
    expect(panel, findsOneWidget);

    // No input fields — not even disabled ones, which read as "temporarily
    // locked" rather than "not a thing the save can do".
    expect(
      find.descendant(of: panel, matching: find.byType(TextField)),
      findsNothing,
    );
    expect(
      find.descendant(of: panel, matching: find.byType(EditableText)),
      findsNothing,
    );
    // No picker, no "reset to spawn", no action of any kind.
    expect(
      find.descendant(of: panel, matching: find.byType(ButtonStyleButton)),
      findsNothing,
    );
    expect(find.textContaining('Choose location'), findsNothing);

    // And no pending edit or validation block reached the notifier.
    final state = positionContainer(tester).read(editorProvider);
    expect(state.pendingEdits, isEmpty);
    expect(state.invalidEditKeys, isEmpty);
    expect(state.hasUnsavedEdits, isFalse);
    expect(find.textContaining(RegExp(r'Save \(')), findsNothing);

    // No write ever leaves this panel.
    expect(
      core.requests.where((r) => r.command == 'write_save'),
      isEmpty,
    );
  });
}
