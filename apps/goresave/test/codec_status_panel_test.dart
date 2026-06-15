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
    // Details labels the backend as the game codec host, not the internal
    // pure-Rust fallback, matching the game-codec headline.
    expect(find.text('Backend: G1R codec host'), findsOneWidget);
    expect(find.textContaining('pure_rust_kraken'), findsNothing);
  });

  testWidgets('warn severity uses a warning icon, not the success check',
      (tester) async {
    const codec = CodecStatus(
      available: true,
      status: 'codec_host_decompress_ready',
      message: 'decode only',
      userSeverity: 'warn',
      userTitle: 'Game codec partly ready',
      userMessage: "The editor can read this game's saves, but saving isn't verified yet.",
      canDecompress: true,
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: codec, codecError: null))),
    ));

    expect(find.text('Game codec partly ready'), findsOneWidget);
    expect(find.byIcon(Icons.warning_amber_rounded), findsOneWidget);
    // Not the fully-ready success check.
    expect(find.byIcon(Icons.check_circle_outline), findsNothing);
  });

  testWidgets('renders codecError once when there is no codec status',
      (tester) async {
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: null, codecError: 'Codec helper not found'))),
    ));

    // Shown exactly once (the parent settings row no longer duplicates it).
    expect(find.text('Codec helper not found'), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
  });

  testWidgets('shows codecError alongside an existing codec status',
      (tester) async {
    const codec = CodecStatus(
      available: true,
      status: 'codec_host_ready',
      message: 'ok',
      userSeverity: 'ok',
      userTitle: 'Game codec ready',
      canCompress: true,
      canDecompress: true,
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(
          codec: codec,
          codecError: 'Codec verification failed',
        ))),
    ));

    // Both the error (e.g. a failed verifyCodec) and the status are visible.
    expect(find.text('Codec verification failed'), findsOneWidget);
    expect(find.text('Game codec ready'), findsOneWidget);
  });
}
