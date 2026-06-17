import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/ui/editor_page.dart' show CodecStatusView;

void main() {
  testWidgets('ready status shows the ready indicator and no setup prompt',
      (tester) async {
    const codec = CodecStatus(
      backend: 'ooz_kraken',
      available: true,
      status: 'ready',
      canDecompress: true,
      canCompress: true,
      adapter: 'ooz_kraken',
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: codec, codecError: null))),
    ));

    expect(find.text('Codec ready'), findsOneWidget);
    expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
    // No game-executable / codec-host configuration call to action.
    expect(find.textContaining('game executable'), findsNothing);
    expect(find.textContaining('codec host'), findsNothing);

    // Techy backend detail stays hidden until Details is expanded.
    expect(find.textContaining('ooz_kraken'), findsNothing);
    await tester.tap(find.text('Details'));
    await tester.pumpAndSettle();
    expect(find.text('Backend: ooz_kraken'), findsOneWidget);
    expect(find.textContaining('Compress: yes'), findsOneWidget);
  });

  testWidgets('decode_only status uses a warning icon, not the success check',
      (tester) async {
    const codec = CodecStatus(
      backend: 'ooz_kraken',
      available: true,
      status: 'decode_only',
      canDecompress: true,
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: codec, codecError: null))),
    ));

    expect(find.text('Codec read-only'), findsOneWidget);
    expect(find.byIcon(Icons.warning_amber_rounded), findsOneWidget);
    expect(find.byIcon(Icons.check_circle_outline), findsNothing);
  });

  testWidgets('unavailable status uses the error icon', (tester) async {
    const codec = CodecStatus(
      backend: 'ooz_kraken',
      available: false,
      status: 'unavailable',
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: codec, codecError: null))),
    ));

    expect(find.text('Codec unavailable'), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
  });

  testWidgets('renders codecError once when there is no codec status',
      (tester) async {
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(codec: null, codecError: 'Codec check failed'))),
    ));

    expect(find.text('Codec check failed'), findsOneWidget);
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
  });

  testWidgets('shows codecError alongside an existing codec status',
      (tester) async {
    const codec = CodecStatus(
      backend: 'ooz_kraken',
      available: true,
      status: 'ready',
      canCompress: true,
      canDecompress: true,
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: SingleChildScrollView(
        child: CodecStatusView(
          codec: codec,
          codecError: 'Codec roundtrip failed',
        ))),
    ));

    expect(find.text('Codec roundtrip failed'), findsOneWidget);
    expect(find.text('Codec ready'), findsOneWidget);
  });
}
