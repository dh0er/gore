import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../../../support/l10n_test_app.dart';

/// Stateful probe standing in for a pane's editor: its TextField draft lives in
/// widget state, so losing state across sidebar switches is visible.
class _StatefulPaneProbe extends StatefulWidget {
  const _StatefulPaneProbe();

  @override
  State<_StatefulPaneProbe> createState() => _StatefulPaneProbeState();
}

class _StatefulPaneProbeState extends State<_StatefulPaneProbe> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: _controller,
      decoration: const InputDecoration(labelText: 'probe'),
    );
  }
}

HeroAttribute _attribute(String id, String setClass, double value) {
  final prefix = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{Hero}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
  ];
  return HeroAttribute(
    id: id,
    setClass: setClass,
    basePath: [...prefix, 'BaseValue'],
    currentPath: [...prefix, 'CurrentValue'],
    baseValue: value,
    currentValue: value,
  );
}

Finder _heroBaseField(String id) {
  final setClass = switch (id) {
    'MaxHealth' => '/Script/G1R.AttributeSet_Health',
    'Resistance_Fire' => '/Script/G1R.AttributeSet_Resistance',
    'Swampweed' => '/Script/G1R.AttributeSet_Drugs',
    'XPKillOrDefeatBounty' => '/Script/G1R.AttributeSet_LevelProgression',
    _ => throw ArgumentError.value(id, 'id'),
  };
  return find.byKey(ValueKey('hero-attribute:$setClass:$id:base'));
}

// Wrap in a constrained box so LayoutBuilder in rows has a finite width.
// Supplies the app's localization delegates so HeroStatsCard's
// AppLocalizations.of(context) calls resolve (English values in tests).
Widget _wrap(Widget child) => MaterialApp(
  locale: const Locale('en'),
  localizationsDelegates: testLocalizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: SizedBox(width: 800, height: 600, child: child)),
);

/// Helper: build a HeroStatsCard with a simple pending-change collector.
HeroStatsCard _card({
  required Future<HeroAttributesResult> Function() load,
  void Function(List<TypedValueEdit>, String?)? onPendingChanged,
  List<TypedValueEdit> Function()? initialPending,
  bool editable = true,
  Object reloadKey = 'save-1',
  Widget? fallback,
  Widget? skillsSection,
  AttributeLabelResolver? attributeLabel,
}) {
  return HeroStatsCard(
    load: load,
    onPendingChanged: onPendingChanged ?? (_, _) {},
    initialPending: initialPending,
    editable: editable,
    reloadKey: reloadKey,
    fallback: fallback,
    skillsSection: skillsSection,
    attributeLabel: attributeLabel,
  );
}

void main() {
  testWidgets('uses localized row label and generic value-field labels', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          attributeLabel: (id, _) =>
              id == 'MaxHealth' ? 'Localized maximum health' : id,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Localized maximum health'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Base value'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Current value'), findsOneWidget);
    expect(find.textContaining('Localized maximum health base'), findsNothing);
  });

  testWidgets('the breath group says the Diving skill overrides it', (
    tester,
  ) async {
    // The player definition carries its own oxygen values for the Diving tag
    // and the game applies them on every load, so an edit here can silently not
    // survive. The note is the only place the user can learn that.
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxOxygen', '/Script/G1R.AttributeSet_Oxygen', 45),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Once Diving is learned'), findsOneWidget);
  });

  testWidgets('groups the skill does not touch carry no note', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Once Diving is learned'), findsNothing);
  });

  // ---------------------------------------------------------------------------
  // Sidebar visibility / navigation
  // ---------------------------------------------------------------------------

  testWidgets('sidebar shows only non-empty groups', (tester) async {
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Resistance_Fire', '/Script/G1R.AttributeSet_Resistance', 3),
      // No thieving, no advanced.
    ];

    await tester.pumpWidget(
      _wrap(
        _card(load: () async => HeroAttributesResult(attributes: attributes)),
      ),
    );
    await tester.pumpAndSettle();

    // Sidebar entries present for non-empty groups (may also appear in the
    // detail card header, so use findsWidgets not findsOneWidget).
    expect(find.text('Main Stats'), findsWidgets);
    expect(find.text('Resistances'), findsWidgets);
    // Entries absent for empty groups.
    expect(find.text('Thieving'), findsNothing);
    expect(find.text('Advanced'), findsNothing);
    // No outer 'Hero stats' wrapper title.
    expect(find.text('Hero stats'), findsNothing);
    // No per-card save button.
    expect(find.byTooltip('Save hero stats'), findsNothing);
  });

  testWidgets('rehydrates queued player drafts from initialPending', (
    tester,
  ) async {
    // Codex: returning to the Player after editing an NPC must resume from the
    // queued 'heroStats' drafts, not show the on-disk value.
    final attr = _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64);
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(attributes: [attr]),
          initialPending: () => [
            TypedValueEdit(path: attr.basePath!, value: 99),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();

    final field = _heroBaseField('MaxHealth');
    final editable = tester.widget<EditableText>(
      find.descendant(of: field, matching: find.byType(EditableText)),
    );
    // Shows the queued draft (99), not the on-disk base (64).
    expect(editable.controller.text, '99');
  });

  testWidgets('selecting a group shows its rows and hides others', (
    tester,
  ) async {
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Resistance_Fire', '/Script/G1R.AttributeSet_Resistance', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        _card(load: () async => HeroAttributesResult(attributes: attributes)),
      ),
    );
    await tester.pumpAndSettle();

    // Default selection is Main stats — its row is shown.
    expect(_heroBaseField('MaxHealth'), findsOneWidget);
    // Resistances row is not shown yet.
    expect(_heroBaseField('Resistance_Fire'), findsNothing);

    // Switch to Resistances.
    await tester.tap(find.text('Resistances'));
    await tester.pumpAndSettle();

    expect(_heroBaseField('Resistance_Fire'), findsOneWidget);
    expect(_heroBaseField('MaxHealth'), findsNothing);
  });

  // The player's transform editor moved OUT of this card into the Charaktere →
  // Position sub-tab (PositionDetail). It must have exactly one live copy: two
  // would both drive the single 'transform' pending key. Guard that this card
  // never hosts a Position pane again.
  testWidgets('hero sidebar has no Position entry', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Position'), findsNothing);
    expect(_heroBaseField('MaxHealth'), findsOneWidget);
  });

  testWidgets('Advanced is a regular sidebar entry (no ExpansionTile)', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute(
                'XPKillOrDefeatBounty',
                '/Script/G1R.AttributeSet_LevelProgression',
                0,
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Sidebar entry exists (may appear in sidebar AND card header).
    expect(find.text('Advanced'), findsWidgets);

    // A sidebar heading may be two words ("Combat / movement") and the sidebar
    // is narrow, so the label wraps onto a second line rather than truncating.
    final sidebarLabel = tester.widgetList<Text>(find.text('Advanced')).first;
    expect(sidebarLabel.maxLines, 2);
    // Default: only group, so it's selected — row is immediately visible.
    expect(_heroBaseField('XPKillOrDefeatBounty'), findsOneWidget);
    // No ExpansionTile needed.
    expect(find.byType(ExpansionTile), findsNothing);
  });

  // ---------------------------------------------------------------------------
  // onPendingChanged fires with correct edits on valid change
  // ---------------------------------------------------------------------------

  testWidgets('onPendingChanged fires with correct edits on valid change', (
    tester,
  ) async {
    List<TypedValueEdit>? lastEdits;
    String? lastError;
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Resistance_Fire', '/Script/G1R.AttributeSet_Resistance', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(attributes: attributes),
          onPendingChanged: (edits, err) {
            lastEdits = edits;
            lastError = err;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Edit MaxHealth (Main stats — default selected).
    await tester.enterText(_heroBaseField('MaxHealth'), '99');
    await tester.pump();

    expect(lastError, isNull);
    expect(lastEdits, isNotNull);
    expect(lastEdits!.length, 1);
    expect(lastEdits!.single.path.last, 'BaseValue');

    // Switch to Resistances and edit there.
    await tester.tap(find.text('Resistances'));
    await tester.pumpAndSettle();
    await tester.enterText(_heroBaseField('Resistance_Fire'), '10');
    await tester.pump();

    // Both edits must be present.
    expect(lastEdits, hasLength(2));
    final ids = lastEdits!.map((e) => e.path[e.path.length - 2]).toSet();
    expect(ids, containsAll(['{MaxHealth}', '{Resistance_Fire}']));
  });

  testWidgets('onPendingChanged fires with empty+error on invalid field', (
    tester,
  ) async {
    List<TypedValueEdit>? lastEdits;
    String? lastError;

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          onPendingChanged: (edits, err) {
            lastEdits = edits;
            lastError = err;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_heroBaseField('MaxHealth'), 'not a number');
    await tester.pump();

    expect(lastEdits, isEmpty);
    expect(lastError, isNotNull);
    expect(lastError, contains('Invalid number'));
    // Inline error shown in the card.
    expect(find.textContaining('Invalid number'), findsOneWidget);
  });

  testWidgets('onPendingChanged fires with empty edits on revert to original', (
    tester,
  ) async {
    List<TypedValueEdit>? lastEdits;

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          onPendingChanged: (edits, _) {
            lastEdits = edits;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    final field = _heroBaseField('MaxHealth');
    await tester.enterText(field, '99');
    await tester.pump();
    expect(lastEdits, hasLength(1));

    await tester.enterText(field, '64');
    await tester.pump();
    // Reverted to original — no pending.
    expect(lastEdits, isEmpty);
  });

  // ---------------------------------------------------------------------------
  // Pending edit survives sidebar switch and is visible on return
  // ---------------------------------------------------------------------------

  testWidgets('pending edit still visible after switching away and back', (
    tester,
  ) async {
    List<TypedValueEdit>? lastEdits;
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Resistance_Fire', '/Script/G1R.AttributeSet_Resistance', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(attributes: attributes),
          onPendingChanged: (edits, _) => lastEdits = edits,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Edit MaxHealth (Main stats).
    await tester.enterText(_heroBaseField('MaxHealth'), '77');
    await tester.pump();
    expect(lastEdits, hasLength(1));

    // Switch to Resistances.
    await tester.tap(find.text('Resistances'));
    await tester.pumpAndSettle();

    // The MaxHealth field is gone from the tree now.
    expect(_heroBaseField('MaxHealth'), findsNothing);
    // But pending is still there — the callback was not cleared.
    expect(lastEdits, hasLength(1));

    // Switch back to Main stats.
    await tester.tap(find.text('Main Stats'));
    await tester.pumpAndSettle();

    // Pending edit '77' must be visible again.
    expect(
      tester.widget<TextField>(_heroBaseField('MaxHealth')).controller!.text,
      '77',
    );
  });

  // ---------------------------------------------------------------------------
  // Load error / fallback
  // ---------------------------------------------------------------------------

  testWidgets('shows load error inline', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => const HeroAttributesResult(error: 'decode failed'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
  });

  testWidgets('load error renders fallback when provided', (tester) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => const HeroAttributesResult(error: 'decode failed'),
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
    expect(find.text('legacy editor'), findsOneWidget);
  });

  testWidgets('zero attributes render fallback without a save button', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => const HeroAttributesResult(attributes: []),
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('legacy editor'), findsOneWidget);
    // No per-card save button in this architecture.
    expect(find.byTooltip('Save hero stats'), findsNothing);
  });

  testWidgets('save validation error keeps typed editors over the fallback', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_heroBaseField('MaxHealth'), 'not a number');
    await tester.pump();

    // The validation error is shown, but the typed editors stay in place —
    // only a failed load swaps in the fallback.
    expect(find.textContaining('Invalid number'), findsOneWidget);
    expect(find.text('legacy editor'), findsNothing);
    expect(_heroBaseField('MaxHealth'), findsOneWidget);
  });

  testWidgets('correcting an invalid input clears the stale validation error', (
    tester,
  ) async {
    String? lastError;

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          onPendingChanged: (_, err) => lastError = err,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final field = _heroBaseField('MaxHealth');
    await tester.enterText(field, 'not a number');
    await tester.pump();
    expect(find.textContaining('Invalid number'), findsOneWidget);
    expect(lastError, isNotNull);

    // Correct the field back to the original (valid, unchanged) value: the
    // pending is empty and the stale error must disappear.
    await tester.enterText(field, '64');
    await tester.pump();

    expect(find.textContaining('Invalid number'), findsNothing);
    expect(lastError, isNull);
  });

  testWidgets('reloadKey change refreshes row values', (tester) async {
    var loadValue = 64.0;
    var reloadKey = Object();

    Widget buildCard() => _wrap(
      _card(
        load: () async => HeroAttributesResult(
          attributes: [
            _attribute(
              'MaxHealth',
              '/Script/G1R.AttributeSet_Health',
              loadValue,
            ),
          ],
        ),
        reloadKey: reloadKey,
      ),
    );

    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // First load: fields show '64'.
    expect(
      find.descendant(of: find.byType(TextField), matching: find.text('64')),
      findsWidgets,
    );

    // Change reloadKey and load value — simulates a fresh SaveInspection.
    loadValue = 70.0;
    reloadKey = Object();
    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // Fields now show '70'; no stale '64' remains.
    expect(
      find.descendant(of: find.byType(TextField), matching: find.text('70')),
      findsWidgets,
    );
    expect(
      find.descendant(of: find.byType(TextField), matching: find.text('64')),
      findsNothing,
    );
  });

  testWidgets('cleared field reports empty+error via onPendingChanged', (
    tester,
  ) async {
    List<TypedValueEdit>? lastEdits;
    String? lastError;

    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          onPendingChanged: (edits, err) {
            lastEdits = edits;
            lastError = err;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(_heroBaseField('MaxHealth'), '');
    await tester.pump();

    expect(lastEdits, isEmpty);
    expect(lastError, isNotNull);
    expect(lastError, contains('MaxHealth is empty'));
    expect(find.textContaining('MaxHealth is empty'), findsOneWidget);
  });

  testWidgets('injected pane state survives switching sidebar entries', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(
        _card(
          load: () async => HeroAttributesResult(
            attributes: [
              _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            ],
          ),
          skillsSection: const _StatefulPaneProbe(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Skills'));
    await tester.pumpAndSettle();
    await tester.enterText(find.widgetWithText(TextField, 'probe'), '123');
    await tester.pump();

    // Switch away and back: the editor must keep its unsaved draft — its
    // text backs a registered pending edit that would otherwise go stale.
    await tester.tap(find.text('Main Stats'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Skills'));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<TextField>(find.widgetWithText(TextField, 'probe'))
          .controller!
          .text,
      '123',
    );
  });

  testWidgets('sidebar selection survives a reloadKey change', (tester) async {
    var reloadKey = Object();

    Widget buildCard() => _wrap(
      _card(
        load: () async => HeroAttributesResult(
          attributes: [
            _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
            _attribute(
              'Resistance_Fire',
              '/Script/G1R.AttributeSet_Resistance',
              3,
            ),
          ],
        ),
        reloadKey: reloadKey,
      ),
    );

    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // Navigate to Resistances.
    await tester.tap(find.text('Resistances'));
    await tester.pumpAndSettle();
    expect(_heroBaseField('Resistance_Fire'), findsOneWidget);

    // A save produces a fresh SaveInspection upstream — simulate the reload.
    reloadKey = Object();
    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // Still on Resistances, not snapped back to Main stats.
    expect(_heroBaseField('Resistance_Fire'), findsOneWidget);
    expect(_heroBaseField('MaxHealth'), findsNothing);
  });

  // ---------------------------------------------------------------------------
  // Regression: _reload must NOT call onPendingChanged during build
  // ---------------------------------------------------------------------------

  testWidgets('reloadKey change does not call onPendingChanged synchronously '
      '(no provider mutation during build)', (tester) async {
    // Regression guard for finding 1: _reload previously called
    // widget.onPendingChanged(const [], null) synchronously before awaiting
    // the load future. With flutter_riverpod, calling
    // clearPendingEdit/setPendingEdit from initState/didUpdateWidget (the
    // call site for _reload) mutates the StateNotifier during the build
    // phase and throws "Tried to modify a provider while the widget tree
    // was building". The fix: _reload clears only local widget state
    // (_pending map); the notifier centrally clears 'heroStats' in
    // refresh() (event-handler context).
    var callCount = 0;
    var reloadKey = Object();

    Widget buildCard() => _wrap(
      _card(
        load: () async => HeroAttributesResult(
          attributes: [
            _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
          ],
        ),
        onPendingChanged: (_, _) => callCount++,
        reloadKey: reloadKey,
      ),
    );

    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // No exception from initial load.
    expect(tester.takeException(), isNull);

    final countAfterFirstLoad = callCount;

    // Simulate a save: reloadKey changes (new SaveInspection instance).
    reloadKey = Object();
    await tester.pumpWidget(buildCard());
    // pump (not pumpAndSettle) to catch any synchronous mutation during the
    // first build frame triggered by didUpdateWidget → _reload.
    await tester.pump();

    // Must not throw "Tried to modify a provider while tree was building".
    expect(tester.takeException(), isNull);

    // After settling the async load completes; the callback fires when the
    // user changes a field, but NOT synchronously from _reload.
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    // onPendingChanged should not have been called from _reload; it was only
    // called from the initial load completion (count stays the same across
    // the reloadKey-triggered rebuild until the user edits a field).
    expect(callCount, countAfterFirstLoad);
  });

  // ---------------------------------------------------------------------------
  // formatHeroValue unit group
  // ---------------------------------------------------------------------------

  group('formatHeroValue', () {
    test('integer value renders without decimal point', () {
      expect(formatHeroValue(64), '64');
    });

    test('negative half renders as -0.5', () {
      expect(formatHeroValue(-0.5), '-0.5');
    });

    test('0.125 is round-trip safe (not rounded to 0.13)', () {
      expect(formatHeroValue(0.125), '0.125');
    });

    test('null renders as empty string', () {
      expect(formatHeroValue(null), '');
    });
  });
}
