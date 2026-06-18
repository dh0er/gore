import 'dart:io';

import 'package:archive/archive.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/export/domain/export_notifier.dart';
import 'package:gore_mod/export/domain/export_request.dart';
import 'package:path/path.dart' as p;

void main() {
  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 4, newValue: 500,
  );
  const sword = OverrideEntry(
    classId: 'ItMw_1H_Sword_01', field: 'm_Weight', oldValue: 5.0, newValue: 1.5,
  );

  testWidgets('export sends the gore_core schema and writes returned files', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': true,
        'files': {
          'enabled.txt': '',
          'Scripts/main.lua': '-- generated mod\n',
        },
      },
    });

    final tmp = Directory.systemTemp.createTempSync('gore_mod_export_');
    addTearDown(() => tmp.deleteSync(recursive: true));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    final overridesNotifier = container.read(overridesProvider.notifier);
    overridesNotifier.setOverride(apple500);
    overridesNotifier.setOverride(sword);

    final exportNotifier = container.read(exportProvider.notifier);
    await exportNotifier.export(
      request: ExportRequest(
        modName: 'MyBalanceMod',
        targetDir: tmp.path,
        delayMs: 0,
      ),
      overrides: container.read(overridesProvider).entries,
    );

    // generate_mod called once with the schema gore_core accepts: `override`
    // (not `overrides`) and typed value keys.
    final genCalls = fake.calls.where((c) => c.command == 'generate_mod').toList();
    expect(genCalls, hasLength(1));
    // The non-existent validate_override command must not be called.
    expect(fake.calls.where((c) => c.command == 'validate_override'), isEmpty);

    final genPayload = genCalls.first.payload;
    expect(genPayload['meta'], containsPair('name', 'MyBalanceMod'));
    expect(genPayload['meta'], containsPair('delay_ms', 0));
    final sentOverrides = genPayload['override'] as List;
    expect(sentOverrides, hasLength(2));
    final appleEntry = sentOverrides
        .cast<Map>()
        .firstWhere((o) => o['class'] == 'ItFo_Apple');
    expect(appleEntry['value_int'], 500);
    expect(appleEntry.containsKey('value'), isFalse);
    final swordEntry = sentOverrides
        .cast<Map>()
        .firstWhere((o) => o['class'] == 'ItMw_1H_Sword_01');
    expect(swordEntry['value_float'], 1.5);

    // Files were materialized under <targetDir>/<modName>/.
    final modDir = p.join(tmp.path, 'MyBalanceMod');
    expect(File(p.join(modDir, 'enabled.txt')).existsSync(), isTrue);
    expect(
      File(p.join(modDir, 'Scripts/main.lua')).readAsStringSync(),
      '-- generated mod\n',
    );

    final result = container.read(exportProvider).result;
    expect(result?.success, isTrue);
    expect(result?.outputPath, modDir);
  });

  testWidgets('packageAsZip writes a .zip nested under the mod name', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': true,
        'files': {
          'enabled.txt': '',
          'Scripts/main.lua': '-- mod\n',
        },
      },
    });
    final tmp = Directory.systemTemp.createTempSync('gore_mod_zip_');
    addTearDown(() => tmp.deleteSync(recursive: true));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: ExportRequest(
        modName: 'ZipMod',
        targetDir: tmp.path,
        packageAsZip: true,
      ),
      overrides: [apple500],
    );

    final zipPath = p.join(tmp.path, 'ZipMod.zip');
    expect(File(zipPath).existsSync(), isTrue);
    expect(container.read(exportProvider).result?.outputPath, zipPath);

    final archive = ZipDecoder().decodeBytes(File(zipPath).readAsBytesSync());
    final names = archive.files.map((f) => f.name).toSet();
    expect(names, contains('ZipMod/enabled.txt'));
    expect(names, contains('ZipMod/Scripts/main.lua'));
  });

  testWidgets('rejects a path-escaping mod name without writing', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {'ok': true, 'files': {'enabled.txt': ''}},
    });
    final tmp = Directory.systemTemp.createTempSync('gore_mod_escape_');
    addTearDown(() => tmp.deleteSync(recursive: true));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: ExportRequest(modName: '../evil', targetDir: tmp.path),
      overrides: [apple500],
    );

    expect(container.read(exportProvider).result?.success, isFalse);
    expect(container.read(exportProvider).result?.error, contains('Invalid mod name'));
    // generate_mod must not even be called for an invalid name.
    expect(fake.calls, isEmpty);
  });

  testWidgets('export surfaces a generation error and writes nothing', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': false,
        'error': {'code': 'BAD_CONFIG', 'message': 'invalid overrides config'},
      },
    });

    final tmp = Directory.systemTemp.createTempSync('gore_mod_export_err_');
    addTearDown(() => tmp.deleteSync(recursive: true));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: ExportRequest(modName: 'Test', targetDir: tmp.path),
      overrides: [apple500],
    );

    final result = container.read(exportProvider).result;
    expect(result?.success, isFalse);
    expect(result?.error, contains('invalid overrides config'));
    expect(Directory(p.join(tmp.path, 'Test')).existsSync(), isFalse);
  });
}
