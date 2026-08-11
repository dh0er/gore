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

  group('diagnosticGameRootCandidate', () {
    test('forwards an explicit root without touching the filesystem', () {
      const root = r'C:\missing\Gothic Remake';
      expect(diagnosticGameRootCandidate(root), root);
    });

    test('matches status normalization for existing nested selections', () {
      final root = Directory.systemTemp.createTempSync('gore_diagnostic_root');
      addTearDown(() => root.deleteSync(recursive: true));
      final g1r = Directory(p.join(root.path, 'G1R'));
      final nested = Directory(p.join(g1r.path, 'Binaries', 'Win64'));
      nested.createSync(recursive: true);

      expect(diagnosticGameRootCandidate(g1r.path), root.path);
      expect(diagnosticGameRootCandidate(nested.path), root.path);
      expect(
        diagnosticGameRootCandidate(nested.path),
        gameRootFromExe(nested.path),
      );
    });

    test('lexically derives the root from the exact executable shape', () {
      const exe =
          r'C:\missing\Gothic Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe';
      expect(diagnosticGameRootCandidate(exe), r'C:\missing\Gothic Remake');
    });

    test('matches the executable shape case-insensitively', () {
      const exe = r'C:\missing\g1r\BINARIES\win64\g1r-win64-shipping.EXE';
      expect(diagnosticGameRootCandidate(exe), r'C:\missing');
    });

    test('forwards a wrong executable name for native diagnosis', () {
      const wrong = r'C:\missing\G1R\Binaries\Win64\other.exe';
      expect(diagnosticGameRootCandidate(wrong), wrong);
    });

    test('returns null only for null or blank selections', () {
      expect(diagnosticGameRootCandidate(null), isNull);
      expect(diagnosticGameRootCandidate(''), isNull);
      expect(diagnosticGameRootCandidate('   '), isNull);
    });
  });
}
