import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/app/ui/path_tree.dart';
import 'package:gore_mod/textures/domain/texture_index_provider.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';
import 'package:gore_mod/textures/ui/texture_tab.dart';

/// Settings store with a game exe path set so TextureTab proceeds past the
/// "set the game path" hint. The path doesn't exist on disk, so the game root
/// resolves to null and no FFI preview/extract is ever attempted.
class _ExeStore implements UiSettingsStore {
  const _ExeStore();

  @override
  UiSettings read() =>
      const UiSettings(gameExePath: r'C:\gore-test-nonexistent\G1R.exe');

  @override
  void write(UiSettings settings) {}
}

const Map<String, String> _fakeIndex = {
  'Game/Textures/A': 'pkg-a',
  'Game/Textures/B': 'pkg-b',
  'Other/C': 'pkg-c',
};

Widget _app({required bool onlyStaged}) {
  return ProviderScope(
    overrides: [
      uiSettingsStoreProvider.overrideWithValue(const _ExeStore()),
      textureIndexProvider.overrideWith((ref) async => _fakeIndex),
    ],
    child: MaterialApp(
      home: Scaffold(body: TextureTab(onlyStaged: onlyStaged)),
    ),
  );
}

void main() {
  ProviderContainer containerOf(WidgetTester t) =>
      ProviderScope.containerOf(t.element(find.byType(TextureTab)));

  TextureReplacementsNotifier notifierOf(WidgetTester t) =>
      containerOf(t).read(textureReplacementsProvider.notifier);

  Future<void> pumpApp(WidgetTester t, {required bool onlyStaged}) async {
    await t.pumpWidget(_app(onlyStaged: onlyStaged));
    // Let the overridden index future resolve and the data branch build.
    await t.pump();
    await t.pump();
  }

  testWidgets('onlyStaged: tree covers staged paths only; un-stage empties '
      'the view live', (t) async {
    await pumpApp(t, onlyStaged: true);

    // Nothing staged yet — the browser shows the empty hint, not a blank tree.
    expect(find.text('No staged texture replacements.'), findsOneWidget);

    notifierOf(t).setReplacement(
      const TextureReplacement(asset: 'Game/Textures/A', imagePath: 'a.png'),
    );
    await t.pump();

    // Only the staged path's folder chain is in the tree; the caption counts
    // staged paths, not the whole index.
    expect(find.text('No staged texture replacements.'), findsNothing);
    expect(find.text('Game/Textures'), findsOneWidget);
    expect(find.text('Other'), findsNothing);
    expect(find.text('1 textures'), findsOneWidget);

    await t.tap(find.text('Game/Textures'));
    await t.pump();
    expect(find.text('A'), findsOneWidget);
    expect(find.text('B'), findsNothing);

    // Un-stage the only replacement — the view empties live.
    notifierOf(t).remove('Game/Textures/A');
    await t.pump();
    expect(find.text('No staged texture replacements.'), findsOneWidget);
    expect(find.text('Game/Textures'), findsNothing);
  });

  testWidgets('onlyStaged: flat search list matches staged paths only', (
    t,
  ) async {
    await pumpApp(t, onlyStaged: true);
    notifierOf(t).setReplacement(
      const TextureReplacement(asset: 'Game/Textures/A', imagePath: 'a.png'),
    );
    await t.pump();

    // "Textures" matches both A and B in the index, but only A is staged.
    await t.enterText(find.byType(TextField), 'Textures');
    await t.pump();
    expect(find.text('Game/Textures/A'), findsOneWidget);
    expect(find.text('Game/Textures/B'), findsNothing);
    expect(find.text('1 match / 1 total'), findsOneWidget);
  });

  testWidgets('onlyStaged: paths list identity is stable unless the staged '
      'key set changes', (t) async {
    await pumpApp(t, onlyStaged: true);
    notifierOf(t).setReplacement(
      const TextureReplacement(asset: 'Game/Textures/A', imagePath: 'a.png'),
    );
    await t.pump();

    List<String> paths() =>
        t.widget<PathTreeBrowser>(find.byType(PathTreeBrowser)).paths;
    final before = paths();
    expect(before, ['Game/Textures/A']);

    // Re-staging the same asset (new state object, same key set) must not
    // produce a new list — the tree browser caches by list identity.
    notifierOf(t).setReplacement(
      const TextureReplacement(
        asset: 'Game/Textures/A',
        imagePath: 'other.png',
      ),
    );
    await t.pump();
    expect(identical(paths(), before), isTrue);

    // A genuine key-set change rebuilds the list (in index order).
    notifierOf(t).setReplacement(
      const TextureReplacement(asset: 'Game/Textures/B', imagePath: 'b.png'),
    );
    await t.pump();
    final after = paths();
    expect(identical(after, before), isFalse);
    expect(after, ['Game/Textures/A', 'Game/Textures/B']);
  });

  testWidgets('default mode: full index shown regardless of staging', (
    t,
  ) async {
    await pumpApp(t, onlyStaged: false);

    expect(find.text('3 textures'), findsOneWidget);
    expect(find.text('Game/Textures'), findsOneWidget);
    expect(find.text('Other'), findsOneWidget);
    expect(find.text('No staged texture replacements.'), findsNothing);
  });
}
