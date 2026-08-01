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
/// So the panel shows the four triplets in the same fields the player editor
/// uses, all DISABLED, and offers no way to change them. If a future change
/// re-enables a field, adds a picker or queues a pending edit here, this test
/// fails — which is the point.
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

    // Twelve fields: CharacterLocation/Rotation and SpawnLocation/Rotation.
    final values = tester
        .widgetList<TextField>(find.byType(TextField))
        .map((f) => f.controller?.text)
        .toList();
    expect(
      values,
      containsAllInOrder(<String>[
        '11', '12', '13', '14', '15', '16', // current pose
        '21', '22', '23', '24', '25', '26', // spawn reference
      ]),
    );
    expect(find.text('Spawn position (reference)'), findsOneWidget);

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

    // The fields look like the player editor's, but every one is disabled.
    final fields = tester.widgetList<TextField>(
      find.descendant(of: panel, matching: find.byType(TextField)),
    );
    expect(fields, hasLength(12));
    expect(
      fields.every((f) => f.enabled == false),
      isTrue,
      reason: 'a single enabled field would be a silent write path',
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
