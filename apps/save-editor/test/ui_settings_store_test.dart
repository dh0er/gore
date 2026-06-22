import 'dart:io';
import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';

void main() {
  test('JSON UI settings store roundtrips window size and maximized flag', () {
    final temp = Directory.systemTemp.createTempSync('goresave-ui-settings-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\ui_settings.json');

    JsonFileUiSettingsStore(file).write(
      const UiSettings(
        windowSize: Size(1720, 980),
        windowMaximized: true,
      ),
    );

    final reloaded = JsonFileUiSettingsStore(file).read();

    expect(reloaded.windowSize, const Size(1720, 980));
    expect(reloaded.windowMaximized, isTrue);
  });

  test('window size defaults to null and maximized to false', () {
    final settings = UiSettings.fromJson(const {'themeMode': 'dark'});

    expect(settings.windowSize, isNull);
    expect(settings.windowMaximized, isFalse);
  });

  test('rejects invalid persisted window sizes', () {
    for (final json in const [
      {'windowWidth': -100, 'windowHeight': 500},
      {'windowWidth': 0, 'windowHeight': 0},
      {'windowWidth': 'wide', 'windowHeight': 500},
      {'windowWidth': 1280},
    ]) {
      expect(
        UiSettings.fromJson(json).windowSize,
        isNull,
        reason: 'should reject $json',
      );
    }
  });

  test('copyWith can update window state without touching other fields', () {
    const initial = UiSettings(uiScale: 1.5);

    final updated = initial.copyWith(
      windowSize: const Size(1600, 900),
      windowMaximized: true,
    );

    expect(updated.uiScale, 1.5);
    expect(updated.windowSize, const Size(1600, 900));
    expect(updated.windowMaximized, isTrue);
  });
}
