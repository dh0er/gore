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
        deletedSaveRecovery: DeletedSaveRecovery(
          targetPath: r'D:\G1R\Saves\G1R-006.sav',
          backupPath: r'D:\G1R\Saves\goresave_backups\G1R-006.sav.bak.1',
          persistentPostDeleteSha1: 'post-delete',
          deletedSaveSha1: 'deleted-save',
          deletedPersistentSha1: 'deleted-profile',
          message: 'Save deleted; backup created',
        ),
      ),
    );

    final reloaded = JsonFileEditorSettingsStore(file).read();

    expect(reloaded.saveDir, r'D:\G1R\Saves');
    expect(reloaded.externalSavePaths, [r'E:\archive\detached.sav']);
    expect(reloaded.hiddenOtherSavePaths, [r'D:\G1R\Saves\G1R-009.sav']);
    expect(
      reloaded.deletedSaveRecovery?.targetPath,
      r'D:\G1R\Saves\G1R-006.sav',
    );
    expect(reloaded.deletedSaveRecovery?.deletedSaveSha1, 'deleted-save');
    expect(reloaded.deletedSaveRecovery?.fileName, 'G1R-006.sav');
  });

  test('JSON settings store ignores corrupt files', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')..writeAsStringSync('{');

    final settings = JsonFileEditorSettingsStore(file).read();

    expect(settings.saveDir, isNull);
    expect(settings.externalSavePaths, isEmpty);
    expect(settings.hiddenOtherSavePaths, isEmpty);
    expect(settings.deletedSaveRecovery, isNull);
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
    expect(settings.deletedSaveRecovery, isNull);
  });

  test('JSON settings store rejects partial delete recovery tokens', () {
    final temp = Directory.systemTemp.createTempSync('goresave-settings-test-');
    addTearDown(() => temp.deleteSync(recursive: true));
    final file = File('${temp.path}\\settings.json')
      ..writeAsStringSync('''
{
  "deletedSaveRecovery": {
    "targetPath": "D:\\\\G1R\\\\Saves\\\\G1R-006.sav",
    "backupPath": "D:\\\\G1R\\\\Saves\\\\goresave_backups\\\\G1R-006.sav.bak.1"
  }
}
''');

    expect(
      JsonFileEditorSettingsStore(file).read().deletedSaveRecovery,
      isNull,
    );
  });
}
