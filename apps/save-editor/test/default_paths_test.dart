import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:goresave/utils/default_paths.dart';

void main() {
  test('default save root points at the G1R SaveGames folder', () {
    final root = defaultSaveRoot();

    // Regardless of which environment variable resolves it, the default always
    // ends at the game's SaveGames directory.
    expect(root, contains('G1R'));
    expect(root, endsWith('SaveGames'));
  });

  test('migrates the previous Linux relative default to Proton discovery', () {
    expect(
      resolveSaveRoot(
        stored: p.join('G1R', 'Saved', 'SaveGames'),
        fallback: '/home/test/.steam/steam/compatdata/123/pfx/saves',
        isLinux: true,
      ),
      '/home/test/.steam/steam/compatdata/123/pfx/saves',
    );
  });

  test('preserves an explicitly stored save directory', () {
    expect(
      resolveSaveRoot(
        stored: '/mnt/game-saves',
        fallback: '/home/test/.steam/steam/compatdata/123/pfx/saves',
        isLinux: true,
      ),
      '/mnt/game-saves',
    );
  });

  test('Linux default finds saves in a Steam Proton prefix', () {
    if (!Platform.isLinux) return;
    final temporaryDirectory = Directory.systemTemp.createTempSync(
      'gore-save-paths-',
    );
    addTearDown(() => temporaryDirectory.deleteSync(recursive: true));

    final home = temporaryDirectory.path;
    final saveRoot = p.join(
      home,
      '.local',
      'share',
      'Steam',
      'steamapps',
      'compatdata',
      '123456',
      'pfx',
      'drive_c',
      'users',
      'steamuser',
      'AppData',
      'Local',
      'G1R',
      'Saved',
      'SaveGames',
    );
    Directory(saveRoot).createSync(recursive: true);
    File(p.join(saveRoot, 'G1R-001.sav')).writeAsStringSync('save');

    expect(
      defaultSaveRoot(environment: {'HOME': home}, isLinux: true),
      saveRoot,
    );
  });
}
