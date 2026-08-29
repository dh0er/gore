import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/ui/design/app_theme.dart';

void main() {
  test('both themes bound how wide a tooltip may grow', () {
    for (final theme in [buildGoresaveTheme(), buildGoresaveDarkTheme()]) {
      final constraints = theme.tooltipTheme.constraints;
      expect(constraints, isNotNull);
      expect(constraints!.maxWidth, lessThan(double.infinity));
    }
  });

  testWidgets('a long tooltip wraps instead of spanning the window', (
    tester,
  ) async {
    // The game's own attribute descriptions are whole sentences. Unbounded,
    // Flutter lays one out on a single line across the entire window.
    const sentence =
        'Lebenspunkte stellen deine physische Konstitution dar. Je mehr '
        'Lebenspunkte du hast, desto mehr Schaden hältst du aus, bevor du zu '
        'Boden gehst. Mit zunehmender Erfahrung steigen deine Lebenspunkte.';
    await tester.pumpWidget(
      MaterialApp(
        theme: buildGoresaveTheme(),
        home: const Scaffold(
          body: Center(
            child: Tooltip(message: sentence, child: Text('Lebenspunkte')),
          ),
        ),
      ),
    );
    final tooltip = tester.state<TooltipState>(find.byType(Tooltip));
    tooltip.ensureTooltipVisible();
    await tester.pumpAndSettle();

    final rendered = find.text(sentence);
    expect(rendered, findsOneWidget);
    final size = tester.getSize(rendered);
    final maxWidth = buildGoresaveTheme().tooltipTheme.constraints!.maxWidth;
    expect(size.width, lessThanOrEqualTo(maxWidth));
    expect(
      size.height,
      greaterThan(40),
      reason: 'the sentence must wrap onto several lines',
    );
  });
}
