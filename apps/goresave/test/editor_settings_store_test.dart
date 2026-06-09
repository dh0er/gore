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
        codecHostPath: r'D:\goresave\goresave_g1r_codec_host.exe',
        gameExePath:
            r'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      ),
    );

    final reloaded = JsonFileEditorSettingsStore(file).read();

    expect(reloaded.saveDir, r'D:\G1R\Saves');
    expect(reloaded.codecHostPath, r'D:\goresave\goresave_g1r_codec_host.exe');
    expect(
      reloaded.gameExePath,
      r'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
  });

  test('JSON settings store ignores corrupt files', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')..writeAsStringSync('{');

    final settings = JsonFileEditorSettingsStore(file).read();

    expect(settings.saveDir, isNull);
    expect(settings.codecHostPath, isNull);
    expect(settings.gameExePath, isNull);
  });
}
