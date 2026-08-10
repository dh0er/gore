import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/npc_position_fake_core.dart';

/// Unlike the player's transform editor — which silently drops its pending edit
/// on unparseable input — the NPC position panel reports the validation error to
/// the host, which marks the key invalid and BLOCKS the global Save button. A
/// stale-but-valid stored draft must never be written from behind a field the
/// user has just emptied or fat-fingered.
void main() {
  NpcPositionCoreService buildCore() => NpcPositionCoreService({
    'Lizard-A': const FakePose(
      location: (10.0, 11.0, 12.0),
      spawnLocation: (500.0, 600.0, 700.0),
    ),
  });

  bool saveEnabled(WidgetTester tester) {
    final save = find.widgetWithText(FilledButton, 'Save (1)');
    expect(save, findsOneWidget);
    return tester.widget<FilledButton>(save).onPressed != null;
  }

  /// Count-agnostic variant: an invalid key that has no pending edit of its own
  /// still bumps the badge, so the label is not predictable once a second
  /// surface is involved.
  bool saveEnabledAnyCount(WidgetTester tester) {
    final save = find.ancestor(
      of: find.textContaining(RegExp(r'^Save')),
      matching: find.byType(FilledButton),
    );
    expect(save, findsOneWidget);
    return tester.widget<FilledButton>(save).onPressed != null;
  }

  Future<void> selectNpc(WidgetTester tester) async {
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
  }

  testWidgets('clearing a field blocks Save', (tester) async {
    await pumpPositionApp(tester, buildCore());
    await openPositionTab(tester);
    await selectNpc(tester);

    // A valid edit first: Save is reachable.
    await tester.enterText(positionField('location:x'), '111');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    // Clearing the field is almost certainly an accident — Save must lock.
    await tester.enterText(positionField('location:x'), '');
    await tester.pump();
    expect(saveEnabled(tester), isFalse);

    // Correcting it releases the block again.
    await tester.enterText(positionField('location:x'), '222');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);
  });

  testWidgets('an unparseable field blocks Save', (tester) async {
    await pumpPositionApp(tester, buildCore());
    await openPositionTab(tester);
    await selectNpc(tester);

    await tester.enterText(positionField('location:y'), '111');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    await tester.enterText(positionField('location:y'), 'nope');
    await tester.pump();
    expect(saveEnabled(tester), isFalse);
  });

  testWidgets('an out-of-range coordinate blocks Save', (tester) async {
    await pumpPositionApp(tester, buildCore());
    await openPositionTab(tester);
    await selectNpc(tester);

    // The typed write path only rejects non-finite values, so the ±10,000,000
    // guard has to live here.
    await tester.enterText(positionField('location:z'), '1e30');
    await tester.pump();
    expect(saveEnabled(tester), isFalse);
    expect(
      find.textContaining('10,000,000'),
      findsOneWidget,
      reason: 'the range violation is named inline',
    );

    await tester.enterText(positionField('location:z'), '10000000');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);
  });

  testWidgets('a position field going valid leaves an attribute block intact', (
    tester,
  ) async {
    await pumpPositionApp(tester, buildCore());
    await openPositionTab(tester);
    await selectNpc(tester);

    await tester.enterText(positionField('location:x'), '111');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    // The Attribute sub-tab is a `_KeepAliveTab` too, so it stays live once
    // visited and can hold a bad field at the same time as this panel.
    // Register its block through the API that panel uses.
    positionContainer(
      tester,
    ).read(editorProvider.notifier).setNpcEditInvalid('npc.attributes:Lizard-A');
    await tester.pump();
    expect(saveEnabledAnyCount(tester), isFalse);

    await tester.enterText(positionField('location:x'), '');
    await tester.pump();
    expect(saveEnabledAnyCount(tester), isFalse);

    // The guard: `setNpcEditInvalid` clears `invalidNpcEditKey` plus EVERY
    // `npc.attributes:` key on every call, so wiring this panel through it
    // would release the attribute panel's block as a side effect the moment
    // the position field recovered. npc_position_invalid_key_isolation_test
    // pins that contract on the notifier; this pins the WIRING in
    // position_detail.dart, which nothing else would catch.
    await tester.enterText(positionField('location:x'), '222');
    await tester.pump();
    expect(
      saveEnabledAnyCount(tester),
      isFalse,
      reason: 'the attribute panel still holds an invalid field',
    );

    // Blocked by the ATTRIBUTE key — not by a position key left stuck behind.
    final invalid = positionContainer(
      tester,
    ).read(editorProvider).invalidEditKeys;
    expect(invalid, contains('npc.attributes:Lizard-A'));
    expect(invalid, isNot(contains('npc.position:Lizard-A')));

    // …and the attribute panel recovering through its OWN api releases Save.
    positionContainer(
      tester,
    ).read(editorProvider.notifier).setNpcEditInvalid(null);
    await tester.pump();
    expect(saveEnabled(tester), isTrue);
  });
}
