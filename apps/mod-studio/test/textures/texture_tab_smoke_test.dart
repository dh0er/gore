import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/textures/ui/texture_tab.dart';

void main() {
  // Compile + mount smoke test: under `flutter test` the settings store is the
  // in-memory noop, so no game path is set and the tab short-circuits before
  // touching the texture index or FFI.
  testWidgets('shows setup hint when no game path is set', (t) async {
    await t.pumpWidget(
      const ProviderScope(
        child: MaterialApp(home: Scaffold(body: TextureTab())),
      ),
    );
    expect(
      find.text('Set the game path in Settings to browse textures.'),
      findsOneWidget,
    );
  });
}
