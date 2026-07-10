import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:path/path.dart' as p;

/// Points every platform's base-dir env var at [base] so `sharedDataDir`
/// resolves under the temp dir no matter which OS runs the test.
Map<String, String> _envFor(String base) => {
      'LOCALAPPDATA': base,
      'APPDATA': base,
      'HOME': base,
      'XDG_DATA_HOME': base,
    };

void main() {
  test('fills missing files from the legacy gore-tools umbrella', () {
    final tmp = Directory.systemTemp.createTempSync('gore_umbrella_mig');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final env = _envFor(tmp.path);
    final newDir = Directory(sharedDataDir(env));
    final legacy = Directory(p.join(p.dirname(newDir.path), 'gore-tools'));

    // The user's real data sits in the legacy umbrella.
    File(p.join(legacy.path, 'gore-manager', 'ui_settings.json'))
      ..createSync(recursive: true)
      ..writeAsStringSync('{"appLocale":"de"}');
    File(p.join(legacy.path, 'loc_catalog.json'))
      ..createSync(recursive: true)
      ..writeAsStringSync('[]');
    // A file already present in the new umbrella must NOT be overwritten.
    File(p.join(newDir.path, 'gore-manager', 'ui_settings.json'))
      ..createSync(recursive: true)
      ..writeAsStringSync('{"appLocale":"en"}');

    migrateLegacyUmbrellaDir(env);

    expect(
      File(p.join(newDir.path, 'loc_catalog.json')).existsSync(),
      isTrue,
      reason: 'loc cache should be copied into the new umbrella',
    );
    expect(
      File(p.join(newDir.path, 'gore-manager', 'ui_settings.json'))
          .readAsStringSync(),
      contains('"en"'),
      reason: 'existing newer settings must be preserved',
    );
    expect(
      File(p.join(newDir.path, '.migrated-from-gore-tools')).existsSync(),
      isTrue,
    );
  });

  test('is a no-op when the legacy umbrella is absent', () {
    final tmp = Directory.systemTemp.createTempSync('gore_umbrella_none');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final env = _envFor(tmp.path);

    migrateLegacyUmbrellaDir(env);

    expect(
      Directory(sharedDataDir(env)).existsSync(),
      isFalse,
      reason: 'nothing to migrate → the new umbrella is not created',
    );
  });

  test('skips re-migration once the marker exists', () {
    final tmp = Directory.systemTemp.createTempSync('gore_umbrella_marker');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final env = _envFor(tmp.path);
    final newDir = Directory(sharedDataDir(env));
    final legacy = Directory(p.join(p.dirname(newDir.path), 'gore-tools'));

    File(p.join(legacy.path, 'loc_catalog.json'))
      ..createSync(recursive: true)
      ..writeAsStringSync('[]');
    // Marker already present: a prior migration ran.
    File(p.join(newDir.path, '.migrated-from-gore-tools'))
        .createSync(recursive: true);

    migrateLegacyUmbrellaDir(env);

    expect(
      File(p.join(newDir.path, 'loc_catalog.json')).existsSync(),
      isFalse,
      reason: 'marker short-circuits before any copy',
    );
  });
}
