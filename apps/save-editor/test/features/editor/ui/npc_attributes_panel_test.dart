import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart'
    show AttributeLabelResolver;
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/ui/npc_attributes_panel.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../../../support/l10n_test_app.dart';

NpcAttributeRow _row(String key, double base, double current) {
  final prefix = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributeSetsByClass',
    '{/Script/G1R.AttributeSet_Health}',
    'Attributes',
    '{$key}',
  ];
  return NpcAttributeRow(
    key: key,
    base: base,
    current: current,
    basePath: [...prefix, 'BaseValue'],
    currentPath: [...prefix, 'CurrentValue'],
  );
}

Finder _npcBaseField(String id) =>
    find.byKey(ValueKey('npc-attribute:$id:base'));

Finder _npcCurrentField(String id) =>
    find.byKey(ValueKey('npc-attribute:$id:current'));

Widget _wrap(Widget child) => MaterialApp(
  locale: const Locale('en'),
  localizationsDelegates: testLocalizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: SizedBox(width: 800, height: 600, child: child)),
);

NpcAttributesPanel _panel({
  required Future<NpcAttributesResult> Function() load,
  void Function(List<NpcTypedEdit>, String?)? onPendingChanged,
  bool editable = true,
  Object reloadKey = 'npc-1',
  NpcStatusConfig? status,
  AttributeLabelResolver? attributeLabel,
}) {
  return NpcAttributesPanel(
    load: load,
    onPendingChanged: onPendingChanged ?? (_, _) {},
    editable: editable,
    reloadKey: reloadKey,
    status: status,
    attributeLabel: attributeLabel,
  );
}

void main() {
  testWidgets('uses localized row label and generic value-field labels', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async =>
              NpcAttributesResult(attributes: [_row('Health', 10, 8)]),
          attributeLabel: (id, setClass) {
            expect(setClass, '/Script/G1R.AttributeSet_Health');
            return id == 'Health' ? 'Localized health' : id;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Localized health'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Base value'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Current value'), findsOneWidget);
    expect(find.textContaining('Localized health base'), findsNothing);
  });

  testWidgets('selecting an NPC shows its Health row', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async =>
              NpcAttributesResult(attributes: [_row('Health', 0, 0)]),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The attribute key is shown as the row label.
    expect(find.text('Health'), findsWidgets);
    // The editable base + current value fields are present.
    expect(_npcBaseField('Health'), findsOneWidget);
    expect(_npcCurrentField('Health'), findsOneWidget);
  });

  testWidgets('editing a row fires onPendingChanged with a typed edit', (
    tester,
  ) async {
    List<NpcTypedEdit>? lastEdits;
    String? lastError;

    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async =>
              NpcAttributesResult(attributes: [_row('Health', 0, 0)]),
          onPendingChanged: (edits, err) {
            lastEdits = edits;
            lastError = err;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_npcBaseField('Health'), '25.6');
    await tester.pump();

    expect(lastError, isNull);
    expect(lastEdits, isNotNull);
    expect(lastEdits, hasLength(1));
    expect(lastEdits!.single.path.last, 'BaseValue');
    expect(lastEdits!.single.value, 25.6);
  });

  testWidgets('invalid input reports empty edits + an error', (tester) async {
    List<NpcTypedEdit>? lastEdits;
    String? lastError;

    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async =>
              NpcAttributesResult(attributes: [_row('Health', 0, 0)]),
          onPendingChanged: (edits, err) {
            lastEdits = edits;
            lastError = err;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_npcBaseField('Health'), 'not a number');
    await tester.pump();

    expect(lastEdits, isEmpty);
    expect(lastError, isNotNull);
  });

  testWidgets('groups NPC rows into the player sidebar categories', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async => NpcAttributesResult(
            attributes: [_row('Health', 40, 25), _row('Resistance_Fire', 0, 0)],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Both group entries appear in the sidebar (Health → Main stats,
    // Resistance_Fire → Resistances).
    expect(find.text('Main stats'), findsWidgets);
    expect(find.text('Resistances'), findsWidgets);
    // Default selection is Main stats — Health row is shown, Resistance is not.
    expect(_npcBaseField('Health'), findsOneWidget);
    expect(_npcBaseField('Resistance_Fire'), findsNothing);

    // Switch to Resistances — its row shows, Health hides.
    await tester.tap(find.text('Resistances').first);
    await tester.pumpAndSettle();
    expect(_npcBaseField('Resistance_Fire'), findsOneWidget);
    expect(_npcBaseField('Health'), findsNothing);
  });

  testWidgets('NPC-only attributes fall into the Advanced group', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async => NpcAttributesResult(
            attributes: [_row('XPKillOrDefeatBounty', 1, 1)],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Only group present is Advanced (the catch-all), so it's selected and
    // its row is immediately visible.
    expect(find.text('Advanced'), findsWidgets);
    expect(_npcBaseField('XPKillOrDefeatBounty'), findsOneWidget);
  });

  testWidgets('Thieving-only attributes produce NO Thieving group for NPCs', (
    tester,
  ) async {
    // PickPocketing classifies into the Thieving group, but NPCs never have a
    // non-zero thieving value, so the NPC panel must drop the group entirely.
    // A Health row keeps the panel non-empty so we can assert a real sidebar.
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async => NpcAttributesResult(
            attributes: [_row('Health', 40, 25), _row('PickPocketing', 0, 0)],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Main stats is present (Health), but Thieving never surfaces for NPCs.
    expect(find.text('Main stats'), findsWidgets);
    expect(find.text('Thieving'), findsNothing);
    // The PickPocketing row is not reachable (its only group is gone).
    expect(_npcBaseField('PickPocketing'), findsNothing);
  });

  testWidgets('Status row is the FIRST entry of the core (Main stats) group', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async => NpcAttributesResult(
            attributes: [
              _row('Health', 40, 25),
              _row('DamageMultiplier', 1, 1),
            ],
          ),
          status: NpcStatusConfig(
            npcId: 'Lizard-2',
            editable: true,
            reloadKey: 'k',
            load: () async => const NpcActorsPage(
              npcs: [NpcActor(id: 'Lizard-2', isDead: true, hp: 0, maxHp: 50)],
            ),
            onRevive: () {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The core (Main stats) group is selected by default and shows the Status
    // row (label "Status") as the first entry of the core group detail, ABOVE
    // the Health attribute fields (not a separate sidebar pane).
    expect(find.text('Status'), findsOneWidget);
    final statusY = tester.getTopLeft(find.text('Status')).dy;
    final healthFieldY = tester.getTopLeft(_npcBaseField('Health')).dy;
    expect(statusY, lessThan(healthFieldY));
  });

  testWidgets('core group + Status row appear even with no core attributes', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          // No attributes at all, but a status config is supplied.
          load: () async => const NpcAttributesResult(attributes: []),
          status: NpcStatusConfig(
            npcId: 'Lizard-2',
            editable: true,
            reloadKey: 'k',
            load: () async => const NpcActorsPage(
              npcs: [NpcActor(id: 'Lizard-2', isDead: false, hp: 9, maxHp: 9)],
            ),
            onRevive: () {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Main stats group is present and the Status row is shown.
    expect(find.text('Main stats'), findsWidgets);
    expect(find.text('Status'), findsOneWidget);
    expect(find.text('alive'), findsOneWidget);
  });

  testWidgets(
    'Status row Revive registers pending without an immediate write',
    (tester) async {
      var revived = 0;
      await tester.pumpWidget(
        _wrap(
          _panel(
            load: () async =>
                NpcAttributesResult(attributes: [_row('Health', 40, 25)]),
            status: NpcStatusConfig(
              npcId: 'Lizard-2',
              editable: true,
              reloadKey: 'k',
              load: () async => const NpcActorsPage(
                npcs: [
                  NpcActor(id: 'Lizard-2', isDead: true, hp: 0, maxHp: 50),
                ],
              ),
              onRevive: () => revived++,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(FilledButton, 'Revive'));
      await tester.pumpAndSettle();

      // The action was registered via the callback (pending), not written.
      expect(revived, 1);
    },
  );

  testWidgets(
    'dead NPC stays dead + Revive enabled when the summary load misses it',
    (tester) async {
      var revived = 0;
      await tester.pumpWidget(
        _wrap(
          _panel(
            load: () async =>
                NpcAttributesResult(attributes: [_row('Health', 40, 25)]),
            status: NpcStatusConfig(
              npcId: 'Lizard-2',
              editable: true,
              reloadKey: 'k',
              // The summary load succeeds but does NOT contain the selected id,
              // so _statusActor stays null — the row must fall back to knownDead
              // instead of defaulting to alive with Revive disabled.
              load: () async => const NpcActorsPage(npcs: []),
              knownDead: true,
              onRevive: () => revived++,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('dead'), findsOneWidget);
      expect(find.text('alive'), findsNothing);
      // Revive remains reachable for the dead NPC.
      await tester.tap(find.widgetWithText(FilledButton, 'Revive'));
      await tester.pumpAndSettle();
      expect(revived, 1);
    },
  );

  testWidgets('load error shows inline', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _panel(
          load: () async => const NpcAttributesResult(error: 'decode failed'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
  });
}
