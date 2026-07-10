import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';

void main() {
  test('JSON settings store roundtrips configured editor paths', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json');
    final store = JsonFileEditorSettingsStore(file);

    store.write(const EditorSettings(saveDir: r'D:\G1R\Saves'));

    final reloaded = JsonFileEditorSettingsStore(file).read();

    expect(reloaded.saveDir, r'D:\G1R\Saves');
  });

  test('JSON settings store ignores corrupt files', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')..writeAsStringSync('{');

    final settings = JsonFileEditorSettingsStore(file).read();

    expect(settings.saveDir, isNull);
  });
}
