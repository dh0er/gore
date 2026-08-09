import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart'
    show formatHeroValue;
import 'package:goresave/providers/data_providers.dart';

import 'support/npc_position_fake_core.dart';

/// Regression test for the phantom cross-NPC edit bug, position edition: NPC
/// position pending edits must be keyed PER-NPC (`npc.position:$id`) so that
/// editing NPC-A, switching to NPC-B, and editing NPC-B keeps BOTH edits — each
/// applied to the correct NPC on Save — instead of NPC-A's edit silently
/// surviving under a shared key (or being clobbered by NPC-B's). 1:1 with
/// npc_attribute_pending_per_npc_test.dart.
void main() {
  NpcPositionCoreService buildCore() => NpcPositionCoreService({
    'Lizard-A': const FakePose(
      location: (1.0, 2.0, 3.0),
      spawnLocation: (500.0, 600.0, 700.0),
    ),
    'Lizard-B': const FakePose(
      location: (10.0, 20.0, 30.0),
      spawnLocation: (500.0, 600.0, 700.0),
    ),
  });

  testWidgets(
    'editing NPC-A then NPC-B keeps both position edits keyed to their own NPC',
    (tester) async {
      final core = buildCore();
      await pumpPositionApp(tester, core);
      await openPositionTab(tester);

      // Select NPC-A from the shared master list and edit its X. With an empty
      // loc catalog the row subtitle shows the raw GlobalId.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      expect(positionField('location:x'), findsOneWidget);
      await tester.enterText(positionField('location:x'), '111');
      await tester.pump();
      // One pending edit registered (NPC-A).
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // It lives under NPC-A's OWN key, not a shared 'npc.position'.
      expect(
        positionContainer(tester).read(editorProvider).pendingEdits.keys,
        contains('npc.position:Lizard-A'),
      );

      // Switch to NPC-B. selectActor does NOT clear pending edits, and the panel
      // reloads B's pose — but A's edit must remain under its own key.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
      // The field now shows B's saved value, not A's draft.
      expect(positionFieldText(tester, 'location:x'), isNot('111'));

      // Edit NPC-B's X. Now there are two pending edits (A + B).
      await tester.enterText(positionField('location:x'), '222');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

      // Save. The single write_save batch must contain BOTH edits, each
      // targeting its own NPC's distinct GlobalId typed path.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(2));

      Map<String, Object?> editFor(String npcId) {
        return edits.firstWhere((e) {
          final value = e['value'] as Map<String, Object?>;
          final path = (value['path'] as List).cast<String>();
          return path.contains('{$npcId}');
        });
      }

      // Struct writes ride the EXISTING typed setValue command — no new write.
      expect(editFor('Lizard-A')['path'], 'private.typed.setValue');
      final aEdit = editFor('Lizard-A')['value'] as Map<String, Object?>;
      final bEdit = editFor('Lizard-B')['value'] as Map<String, Object?>;
      // NPC-A's edit was NOT lost and NOT clobbered — the untouched axes come
      // from A's own saved pose.
      expect(aEdit['value'], {'x': 111.0, 'y': 2.0, 'z': 3.0});
      expect(bEdit['value'], {'x': 222.0, 'y': 20.0, 'z': 30.0});
      // The whole triplet leaf is addressed, not a scalar member.
      expect((aEdit['path'] as List).last, 'CharacterLocation');
      expect((bEdit['path'] as List).last, 'CharacterLocation');
    },
  );

  testWidgets('selecting an NPC with no position edit leaves no stale entry', (
    tester,
  ) async {
    final core = buildCore();
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);

    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
    await tester.enterText(positionField('location:x'), '111');
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    await tester.tap(find.text('Lizard-B'));
    await tester.pumpAndSettle();

    // Visiting B without editing must not add a pending entry for B.
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(1));
    final value = edits.single['value'] as Map<String, Object?>;
    expect((value['path'] as List).cast<String>(), contains('{Lizard-A}'));
    expect(value['value'], {'x': 111.0, 'y': 2.0, 'z': 3.0});
  });

  // A spot the bundled catalog is already pinned on by catalog_loaders_test.
  // Its name is unique as a substring, so a search resolves to one row.
  const spotName = 'IO_SC_ANVIL_2';

  LocationSpot spotOf(LocationCatalog catalog) =>
      catalog.spots.firstWhere((s) => s.name == spotName);

  testWidgets('picking a location queues the NPC\'s own typed setValue edit', (
    tester,
  ) async {
    final core = buildCore();
    final spot = spotOf(await loadBundledCatalog(tester));
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();

    await pickLocationSpot(tester, spotName);

    // The picker fills the SAME controllers the manual fields drive.
    expect(positionFieldText(tester, 'location:x'), formatHeroValue(spot.x));
    expect(positionFieldText(tester, 'location:y'), formatHeroValue(spot.y));
    expect(positionFieldText(tester, 'location:z'), formatHeroValue(spot.z));
    // Rotation is opt-in; untouched here, so the saved pose stays.
    expect(positionFieldText(tester, 'rotation:yaw'), '0');

    // …and goes through the same per-NPC pending key, not a second route.
    expect(
      positionContainer(tester).read(editorProvider).pendingEdits.keys,
      contains('npc.position:Lizard-A'),
    );
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(1));
    expect(edits.single['path'], 'private.typed.setValue');
    final value = edits.single['value'] as Map<String, Object?>;
    expect((value['path'] as List).cast<String>(), contains('{Lizard-A}'));
    expect((value['path'] as List).last, 'CharacterLocation');
    expect(value['value'], {'x': spot.x, 'y': spot.y, 'z': spot.z});
  });

  testWidgets('opting into the orientation adds a yaw-only rotation edit', (
    tester,
  ) async {
    final core = buildCore();
    final spot = spotOf(await loadBundledCatalog(tester));
    await pumpPositionApp(tester, core);
    await openPositionTab(tester);
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();

    await pickLocationSpot(tester, spotName, applyRotation: true);

    expect(positionFieldText(tester, 'rotation:pitch'), '0');
    expect(positionFieldText(tester, 'rotation:yaw'), formatHeroValue(spot.yaw));
    expect(positionFieldText(tester, 'rotation:roll'), '0');

    // Still ONE pending key for this NPC, carrying both differing groups (the
    // badge counts EDITS, so a single key with two groups reads as 2).
    expect(
      positionContainer(tester).read(editorProvider).pendingEdits.keys,
      ['npc.position:Lizard-A'],
    );
    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(2));
    final rotation = edits.firstWhere((e) {
      final value = e['value'] as Map<String, Object?>;
      return (value['path'] as List).last == 'CharacterRotation';
    });
    // The catalog carries yaw only: pitch and roll are zeroed, not invented.
    expect((rotation['value'] as Map<String, Object?>)['value'], {
      'pitch': 0.0,
      'yaw': spot.yaw,
      'roll': 0.0,
    });
  });
}
