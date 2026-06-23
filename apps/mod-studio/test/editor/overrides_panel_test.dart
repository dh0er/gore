import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/editor/ui/overrides_panel.dart';
import 'package:gore_mod/l10n/app_localizations.dart';

void main() {
  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 4, newValue: 500,
  );
  const sword = OverrideEntry(
    classId: 'ItMw_1H_Sword_01', field: 'm_Value', oldValue: 50, newValue: 200,
  );

  Widget buildPanel({List<OverrideEntry> initial = const []}) {
    return ProviderScope(
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Consumer(
            builder: (context, ref, _) {
              // Seed initial overrides
              if (initial.isNotEmpty) {
                WidgetsBinding.instance.addPostFrameCallback((_) {
                  final notifier = ref.read(overridesProvider.notifier);
                  for (final e in initial) {
                    notifier.setOverride(e);
                  }
                });
              }
              return const OverridesPanel();
            },
          ),
        ),
      ),
    );
  }

  testWidgets('shows empty state message when no overrides', (tester) async {
    await tester.pumpWidget(buildPanel());
    await tester.pump();
    expect(find.text('No pending overrides.\nEdit item fields to add some.'), findsOneWidget);
  });

  testWidgets('shows override rows when overrides present', (tester) async {
    await tester.pumpWidget(buildPanel(initial: [apple500, sword]));
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Apple.m_Value'),      findsOneWidget);
    expect(find.text('ItMw_1H_Sword_01.m_Value'), findsOneWidget);
    expect(find.text('4 → 500'),  findsOneWidget);
    expect(find.text('50 → 200'), findsOneWidget);
  });

  testWidgets('remove button removes an override', (tester) async {
    await tester.pumpWidget(buildPanel(initial: [apple500]));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.remove_circle_outline));
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Apple.m_Value'), findsNothing);
  });

  testWidgets('Clear all button removes all overrides', (tester) async {
    await tester.pumpWidget(buildPanel(initial: [apple500, sword]));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Clear all'));
    await tester.pumpAndSettle();
    expect(find.text('No pending overrides.\nEdit item fields to add some.'), findsOneWidget);
  });

  testWidgets('count in header matches change count', (tester) async {
    await tester.pumpWidget(buildPanel(initial: [apple500, sword]));
    await tester.pumpAndSettle();
    expect(find.text('Changes (2)'), findsOneWidget);
  });
}
