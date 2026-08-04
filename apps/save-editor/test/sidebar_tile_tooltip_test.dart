import 'package:flutter/gestures.dart' show PointerDeviceKind;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';

/// A sidebar row ellipsizes its label, so a long area name like
/// "Illegale Sumpfkrautmischer (109)" is unreadable in the location picker's
/// rail. Hovering must reveal the full name — but only when it is actually cut
/// off: a tooltip repeating a label that already fits is pure noise.
void main() {
  Future<void> pumpTile(WidgetTester tester, String label, double width) {
    return tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: width,
            child: SidebarTile(
              icon: Icons.place_outlined,
              label: label,
              selected: false,
              onTap: () {},
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('a truncated label gets a tooltip carrying the full text', (
    tester,
  ) async {
    const label = 'Illegale Sumpfkrautmischer (109)';
    await pumpTile(tester, label, 120);

    final tooltip = tester.widget<Tooltip>(find.byType(Tooltip));
    expect(tooltip.message, label);
  });

  testWidgets('a label that fits gets no tooltip', (tester) async {
    await pumpTile(tester, 'Tundra (213)', 400);

    expect(find.byType(Tooltip), findsNothing);
  });

  testWidgets('the tooltip shows the full name on hover', (tester) async {
    const label = 'Illegale Sumpfkrautmischer (109)';
    await pumpTile(tester, label, 120);

    // Before hovering the name is only present as the ellipsized Text.
    expect(find.text(label), findsOneWidget);

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();
    await gesture.moveTo(tester.getCenter(find.byType(SidebarTile)));
    await tester.pump();
    // Tooltip has a wait duration before it appears on hover.
    await tester.pump(const Duration(seconds: 1));

    // Now the label is rendered twice: the row itself and the overlay.
    expect(find.text(label), findsNWidgets(2));
  });
}
