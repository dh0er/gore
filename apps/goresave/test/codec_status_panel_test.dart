import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/ui/editor_page.dart' show CodecStatusView;

void main() {
  testWidgets('shows plain message + hint, hides techy detail behind Details',
      (tester) async {
    const codec = CodecStatus(
      available: false,
      status: 'unsupported',
      message: 'G1R codec host is configured but not available.',
      userSeverity: 'error',
      userTitle: "This game version can't be opened yet",
      userMessage: "Looks like a new game update the editor doesn't recognize yet.",
      userHint: 'Check for an editor update - a new version usually follows shortly.',
      profile: 'g1r-23A85CE7',
      resolutionMode: 'pattern_profile',
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: codec, codecError: null))),
    ));

    expect(find.text("This game version can't be opened yet"), findsOneWidget);
    expect(find.textContaining('editor update'), findsOneWidget);
    // Techy field hidden until Details expanded.
    expect(find.textContaining('pattern_profile'), findsNothing);

    await tester.tap(find.text('Details'));
    await tester.pumpAndSettle();
    expect(find.textContaining('pattern_profile'), findsOneWidget);
  });
}
