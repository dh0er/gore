import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/textures/domain/texture_index_provider.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';
import 'package:gore_mod/textures/ui/texture_tab.dart';

/// Settings store with a game exe path set so TextureTab proceeds past the
/// "set the game path" hint. The path doesn't exist on disk, so the game root
/// resolves to null and no FFI preview/extract is ever attempted — the
/// original-preview branch stays on its "Preview to see…" placeholder, making
/// the staged-vs-original branch switch directly observable.
class _ExeStore implements UiSettingsStore {
  const _ExeStore();

  @override
  UiSettings read() =>
      const UiSettings(gameExePath: r'C:\gore-test-nonexistent\G1R.exe');

  @override
  void write(UiSettings settings) {}
}

const _asset = 'Game/Textures/A';
const Map<String, String> _fakeIndex = {_asset: 'pkg-a'};

/// A valid 1×1 transparent PNG (the classic kTransparentImage bytes).
final Uint8List _onePixelPng = Uint8List.fromList(const [
  0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, //
  0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, //
  0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, //
  0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, //
  0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, //
  0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, //
]);

Widget _app() => ProviderScope(
  overrides: [
    uiSettingsStoreProvider.overrideWithValue(const _ExeStore()),
    textureIndexProvider.overrideWith((ref) async => _fakeIndex),
  ],
  child: const MaterialApp(home: Scaffold(body: TextureTab())),
);

void main() {
  late Directory tempDir;

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('gore-staged-preview-');
  });

  tearDown(() {
    try {
      tempDir.deleteSync(recursive: true);
    } catch (_) {
      // Best-effort cleanup of the temp PNG dir.
    }
  });

  TextureReplacementsNotifier notifierOf(WidgetTester t) =>
      ProviderScope.containerOf(
        t.element(find.byType(TextureTab)),
      ).read(textureReplacementsProvider.notifier);

  /// An [Image] whose provider is a [FileImage] on exactly [path].
  Finder fileImage(String path) => find.byWidgetPredicate(
    (w) =>
        w is Image &&
        w.image is FileImage &&
        (w.image as FileImage).file.path == path,
  );

  /// Pump the app, let the index future resolve, and select [_asset] via the
  /// flat search list (no FFI extract runs: the game root is null).
  Future<void> pumpAndSelect(WidgetTester t) async {
    await t.pumpWidget(_app());
    await t.pump();
    await t.pump();
    await t.enterText(find.byType(TextField), _asset);
    await t.pump();
    await t.tap(find.byType(ListTile).first);
    await t.pump();
  }

  testWidgets('staged replacement PNG takes precedence over the original '
      'preview; Remove reverts', (t) async {
    final png = File('${tempDir.path}/repl.png')
      ..writeAsBytesSync(_onePixelPng);
    await pumpAndSelect(t);

    // Nothing staged: the original branch (placeholder — no FFI in tests),
    // no badge, no staged file image.
    expect(find.text('Preview to see the current texture'), findsOneWidget);
    expect(find.text('Replacement'), findsNothing);
    expect(fileImage(png.path), findsNothing);

    // Stage a replacement → the detail preview shows the STAGED PNG (a
    // FileImage on exactly that path) with the 'Replacement' badge, not the
    // native preview branch.
    notifierOf(t).setReplacement(
      TextureReplacement(asset: _asset, imagePath: png.path),
    );
    await t.pump();
    expect(find.text('Replacement'), findsOneWidget);
    expect(fileImage(png.path), findsOneWidget);
    expect(find.text('Preview to see the current texture'), findsNothing);

    // Remove the replacement → the staged branch yields (badge + staged image
    // gone) and the original branch shows again (its placeholder here, since
    // no native extract ever ran in the test environment).
    notifierOf(t).remove(_asset);
    await t.pump();
    expect(find.text('Replacement'), findsNothing);
    expect(fileImage(png.path), findsNothing);
    expect(find.text('Preview to see the current texture'), findsOneWidget);
  });

  testWidgets('missing staged PNG falls back to the original with a hint', (
    t,
  ) async {
    await pumpAndSelect(t);

    final missing = '${tempDir.path}/does-not-exist.png';
    notifierOf(t).setReplacement(
      TextureReplacement(asset: _asset, imagePath: missing),
    );
    await t.pump();

    // Fallback: hint + original branch, no badge, no staged image.
    expect(find.text('Staged PNG missing — showing original'), findsOneWidget);
    expect(find.text('Replacement'), findsNothing);
    expect(fileImage(missing), findsNothing);
    expect(find.text('Preview to see the current texture'), findsOneWidget);

    // Removing the broken replacement clears the hint too.
    notifierOf(t).remove(_asset);
    await t.pump();
    expect(find.text('Staged PNG missing — showing original'), findsNothing);
    expect(find.text('Preview to see the current texture'), findsOneWidget);
  });
}
