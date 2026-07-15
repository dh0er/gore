import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';

void main() {
  test('JSON settings store roundtrips configured editor paths', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json');
    final store = JsonFileEditorSettingsStore(file);

    store.write(
      const EditorSettings(
        saveDir: r'D:\G1R\Saves',
        externalSavePaths: [r'E:\archive\detached.sav'],
        hiddenOtherSavePaths: [r'D:\G1R\Saves\G1R-009.sav'],
      ),
    );

    final reloaded = JsonFileEditorSettingsStore(file).read();

    expect(reloaded.saveDir, r'D:\G1R\Saves');
    expect(reloaded.externalSavePaths, [r'E:\archive\detached.sav']);
    expect(reloaded.hiddenOtherSavePaths, [r'D:\G1R\Saves\G1R-009.sav']);
  });

  test('JSON settings store ignores corrupt files', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')..writeAsStringSync('{');

    final settings = JsonFileEditorSettingsStore(file).read();

    expect(settings.saveDir, isNull);
    expect(settings.externalSavePaths, isEmpty);
    expect(settings.hiddenOtherSavePaths, isEmpty);
  });

  test('JSON settings store sanitizes persisted save path lists', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')
      ..writeAsStringSync('''
{
  "externalSavePaths": ["  D:\\\\one.sav  ", "", 7, "D:\\\\one.sav"],
  "hiddenOtherSavePaths": "invalid"
}
''');

    final settings = JsonFileEditorSettingsStore(file).read();

    expect(settings.externalSavePaths, [r'D:\one.sav']);
    expect(settings.hiddenOtherSavePaths, isEmpty);
  });
}
