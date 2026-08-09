import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/npc_position_fake_core.dart';

/// Moving an NPC only sticks if his daily routine is replaced at the same time.
///
/// `CharacterLocation` alone IS applied on load — the NPC really appears where
/// he was put — and then his routine walks him back within seconds. So the
/// position write and the routine write are one action and must ride one Save;
/// a position that lands without its routine is the bug this file guards.
void main() {
  const npc = 'Lizard-A';

  NpcPositionCoreService coreWith({String? routineClass, FakeUndo? undo}) =>
      NpcPositionCoreService({
        npc: FakePose(
          location: const (11.0, 12.0, 13.0),
          rotation: const (14.0, 15.0, 16.0),
          spawnLocation: const (21.0, 22.0, 23.0),
          spawnRotation: const (24.0, 25.0, 26.0),
          routineClass: routineClass,
          undo: undo,
        ),
      });

  Future<void> openNpc(WidgetTester tester, NpcPositionCoreService core) async {
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await tester.tap(find.text(npc));
    await tester.pumpAndSettle();
  }

  PendingSaveEdit? pending(WidgetTester tester) => positionContainer(
    tester,
  ).read(editorProvider).pendingEdits['npc.position:$npc'];

  List<Map<String, Object?>> editsOf(WidgetTester tester) =>
      pending(tester)?.edits ?? const [];

  Map<String, Object?>? routineEdit(WidgetTester tester) {
    for (final edit in editsOf(tester)) {
      final value = edit['value'];
      if (value is! Map) continue;
      final path = value['path'];
      if (path is List && path.last == 'DailyRoutineClass') {
        return edit.cast<String, Object?>();
      }
    }
    return null;
  }

  testWidgets('moving an NPC also queues the inert routine, and a note', (
    tester,
  ) async {
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    await tester.enterText(positionField('location:x'), '999');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();

    final routine = routineEdit(tester);
    expect(routine, isNotNull, reason: 'the routine write must ride along');
    expect(
      (routine!['value'] as Map)['value'],
      kFakeInertRoutine,
      reason: 'every other routine class walks the NPC back',
    );
    expect((routine['value'] as Map)['path'], routinePath(npc));

    // Both writes are in ONE pending entry, so one Save carries both.
    expect(editsOf(tester).length, 2);

    final notes = pending(tester)!.placementNotes;
    expect(notes.length, 1);
    expect(notes.single['npc'], npc);
    final note = (notes.single['note'] as Map).cast<String, Object?>();
    expect(
      note['original_routine_class'],
      '/Script/Angelscript.DailyRoutine_A_Start',
      reason: 'the replaced class is not derivable, so it must be written down',
    );
    expect(note['original_location'], [11.0, 12.0, 13.0]);
    expect(note['written_location'], [999.0, 12.0, 13.0]);
    expect(note['written_routine_class'], kFakeInertRoutine);
  });

  testWidgets('the box shows the stored state and starts OFF, not ticked', (
    tester,
  ) async {
    // It is a state ("has a daily routine"), not a modifier on a pending move,
    // so it is always there — and off, because wiping a schedule must never be a
    // side effect of editing a coordinate.
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    final box = tester.widget<CheckboxListTile>(
      find.byKey(const ValueKey('npc-position:stay')),
    );
    expect(box.value, isFalse);
    expect(box.onChanged, isNotNull);
    expect(pending(tester), isNull, reason: 'showing it queues nothing');
  });

  testWidgets('ticking it does not lock it — nothing is written yet', (
    tester,
  ) async {
    // Regression: the lock was gated on the tick instead of on the stored state,
    // so every NPC greyed out the moment the box was ticked, telling the user
    // there was no way back from a change that had not even been saved.
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();

    final ticked = tester.widget<CheckboxListTile>(
      find.byKey(const ValueKey('npc-position:stay')),
    );
    expect(ticked.value, isTrue);
    expect(ticked.onChanged, isNotNull, reason: 'unticking must stay possible');

    // And unticking really does take it back, without writing anything.
    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();
    expect(routineEdit(tester), isNull);
    expect(pending(tester), isNull);
  });

  testWidgets('ticking it alone freezes him where he stands', (tester) async {
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();

    final routine = routineEdit(tester);
    expect(routine, isNotNull);
    expect((routine!['value'] as Map)['value'], kFakeInertRoutine);
    expect(editsOf(tester).length, 1, reason: 'no position edit was made');
    final note = (pending(tester)!.placementNotes.single['note'] as Map)
        .cast<String, Object?>();
    expect(note['written_location'], [11.0, 12.0, 13.0]);
  });

  testWidgets('leaving the box alone moves him without touching his routine', (
    tester,
  ) async {
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    await tester.enterText(positionField('location:x'), '999');
    await tester.pumpAndSettle();

    expect(routineEdit(tester), isNull);
    expect(editsOf(tester).length, 1);
    expect(pending(tester)!.placementNotes, isEmpty);
  });

  testWidgets('a queued pin survives leaving the NPC and coming back', (
    tester,
  ) async {
    // Regression: the box was reseeded from the SAVE on every revisit while the
    // queue still held the routine swap, so the next keystroke rebuilt the entry
    // without it and the move saved as position-only — the one shape that does
    // not stick.
    await pumpPositionApp(
      tester,
      NpcPositionCoreService({
        npc: const FakePose(
          location: (11.0, 12.0, 13.0),
          routineClass: '/Script/Angelscript.DailyRoutine_A_Start',
        ),
        'Lizard-B': const FakePose(location: (1.0, 2.0, 3.0)),
      }),
    );
    await openPositionTab(tester);
    await tester.tap(find.text(npc));
    await tester.pumpAndSettle();

    await tester.enterText(positionField('location:x'), '999');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();
    expect(routineEdit(tester), isNotNull);

    // Away and back.
    await tester.tap(find.text('Lizard-B'));
    await tester.pumpAndSettle();
    await tester.tap(find.text(npc));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey('npc-position:stay')),
          )
          .value,
      isTrue,
      reason: 'the box has to resume from the queue, not from the save',
    );

    // The keystroke that used to drop it.
    await tester.enterText(positionField('location:y'), '888');
    await tester.pumpAndSettle();

    final routine = routineEdit(tester);
    expect(routine, isNotNull, reason: 'the pin must still be queued');
    expect((routine!['value'] as Map)['value'], kFakeInertRoutine);
    expect(pending(tester)!.placementNotes, hasLength(1));
  });

  testWidgets('an NPC with no routine record offers no checkbox at all', (
    tester,
  ) async {
    await openNpc(tester, coreWith(routineClass: null));

    expect(find.byKey(const ValueKey('npc-position:stay')), findsNothing);

    await tester.enterText(positionField('location:x'), '999');
    await tester.pumpAndSettle();

    expect(editsOf(tester).length, 1, reason: 'the position still moves');
    expect(pending(tester)!.placementNotes, isEmpty);
  });

  testWidgets('taking the move back restores the position and the routine', (
    tester,
  ) async {
    await openNpc(
      tester,
      coreWith(routineClass: kFakeInertRoutine, undo: const FakeUndo()),
    );

    await tester.tap(find.byKey(const ValueKey('npc-position:undo')));
    await tester.pumpAndSettle();

    expect(positionFieldText(tester, 'location:x'), '7');
    final routine = routineEdit(tester);
    expect(routine, isNotNull);
    expect(
      (routine!['value'] as Map)['value'],
      '/Script/Angelscript.DailyRoutine_A_Start',
      reason: 'the restore writes the recorded class, not the inert one',
    );
    expect(
      pending(tester)!.clearPlacementNotes,
      [npc],
      reason: 'a spent note must not offer a second restore',
    );
    expect(pending(tester)!.placementNotes, isEmpty);
  });

  testWidgets('a move that turns him records and restores the facing too', (
    tester,
  ) async {
    // The picker can apply a spot's heading. Restoring the position while
    // leaving the new facing would strand it: clearing the note takes the only
    // record of the old one with it.
    await openNpc(
      tester,
      coreWith(routineClass: '/Script/Angelscript.DailyRoutine_A_Start'),
    );

    await tester.enterText(positionField('location:x'), '999');
    await tester.enterText(positionField('rotation:yaw'), '90');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('npc-position:stay')));
    await tester.pumpAndSettle();

    final note = (pending(tester)!.placementNotes.single['note'] as Map)
        .cast<String, Object?>();
    expect(note['original_rotation'], [14.0, 15.0, 16.0]);
    expect(note['written_rotation'], [14.0, 90.0, 16.0]);

    // And a restore puts that facing back.
    await pumpPositionApp(
      tester,
      coreWith(
        routineClass: kFakeInertRoutine,
        undo: const FakeUndo(originalRotation: (14.0, 15.0, 16.0)),
      ),
    );
    await openPositionTab(tester);
    await tester.tap(find.text(npc));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('npc-position:undo')));
    await tester.pumpAndSettle();

    expect(positionFieldText(tester, 'rotation:yaw'), '15');
  });

  testWidgets('a note the save no longer matches cannot be restored', (
    tester,
  ) async {
    await openNpc(
      tester,
      coreWith(
        routineClass: kFakeInertRoutine,
        undo: const FakeUndo(restorable: false),
      ),
    );

    final button = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('npc-position:undo')),
    );
    expect(
      button.onPressed,
      isNull,
      reason: 'restoring over a changed save would discard what happened since',
    );
  });

  testWidgets('an NPC already on the inert routine queues no second swap', (
    tester,
  ) async {
    await openNpc(tester, coreWith(routineClass: kFakeInertRoutine));

    // Ticked, because that IS his state — and locked, because giving the routine
    // back needs a recorded class and this NPC has no note behind him.
    final box = tester.widget<CheckboxListTile>(
      find.byKey(const ValueKey('npc-position:stay')),
    );
    expect(box.value, isTrue);
    expect(box.onChanged, isNull);

    await tester.enterText(positionField('location:x'), '999');
    await tester.pumpAndSettle();

    expect(routineEdit(tester), isNull);
    expect(editsOf(tester).length, 1);
    expect(pending(tester)!.placementNotes, isEmpty);
  });
}
