import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/game_paths.dart';
import 'package:path/path.dart' as p;

void main() {
  group('gameRootFromExe', () {
    test('resolves an install root (a dir holding G1R/) to itself', () {
      // The shared game_path from `gore config set game-path`/`detect` is the
      // install root, not an exe. It must resolve to itself, not its parent.
      final dir = Directory.systemTemp.createTempSync('gore_root');
      addTearDown(() => dir.deleteSync(recursive: true));
      Directory(p.join(dir.path, 'G1R')).createSync();

      expect(gameRootFromExe(dir.path), dir.path);
    });

    test('resolves the game .exe to its install root', () {
      final dir = Directory.systemTemp.createTempSync('gore_root');
      addTearDown(() => dir.deleteSync(recursive: true));
      final exe = p.join(
        dir.path,
        'G1R',
        'Binaries',
        'Win64',
        'G1R-Win64-Shipping.exe',
      );
      Directory(p.dirname(exe)).createSync(recursive: true);
      File(exe).writeAsStringSync('x');

      expect(gameRootFromExe(exe), dir.path);
    });

    test('returns null for null or empty', () {
      expect(gameRootFromExe(null), isNull);
      expect(gameRootFromExe(''), isNull);
    });
  });
}
