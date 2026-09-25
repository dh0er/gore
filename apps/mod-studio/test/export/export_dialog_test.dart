import 'dart:io';

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

  testWidgets('export writes a source spec and not a finished mod', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': true,
        'files': {
          'spec.json': '{"meta":{"name":"MyBalanceMod"},"values":[]}\n',
        },
        'note': 'item values compile into a script mini-cache via gore mod build',
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
    expect((genPayload['meta'] as Map).containsKey('delay_ms'), isFalse);
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

    final specPath = p.join(tmp.path, 'MyBalanceMod.spec.json');
    expect(File(specPath).readAsStringSync(), contains('"values"'));
    expect(Directory(p.join(tmp.path, 'MyBalanceMod')).existsSync(), isFalse);
    expect(File(p.join(tmp.path, 'MyBalanceMod.zip')).existsSync(), isFalse);

    final result = container.read(exportProvider).result;
    expect(result?.success, isTrue);
    expect(result?.outputPath, specPath);
    expect(result?.note, contains('gore mod build'));
  });

  testWidgets('a lua-shaped files map is not written as a mod', (tester) async {
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
      request: ExportRequest(modName: 'ZipMod', targetDir: tmp.path),
      overrides: [apple500],
    );

    expect(container.read(exportProvider).result?.success, isFalse);
    expect(File(p.join(tmp.path, 'ZipMod.zip')).existsSync(), isFalse);
    expect(Directory(p.join(tmp.path, 'ZipMod')).existsSync(), isFalse);
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
    // '../evil' contains a path separator and is rejected with a human-readable
    // message (not the raw enum name).
    expect(
      container.read(exportProvider).result?.error,
      contains('must not contain'),
    );
    // generate_mod must not even be called for an invalid name.
    expect(fake.calls, isEmpty);
  });

  testWidgets('a non-spec response writes nothing', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': true,
        'files': {
          'enabled.txt': '',
          'bad?name.txt': 'x',
        },
      },
    });
    final tmp = Directory.systemTemp.createTempSync('gore_mod_partial_');
    addTearDown(() => tmp.deleteSync(recursive: true));

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: ExportRequest(modName: 'PartialMod', targetDir: tmp.path),
      overrides: [apple500],
    );

    expect(container.read(exportProvider).result?.success, isFalse);
    expect(Directory(p.join(tmp.path, 'PartialMod')).existsSync(), isFalse);
    expect(File(p.join(tmp.path, 'PartialMod.spec.json')).existsSync(), isFalse);
  });

  testWidgets('a non-spec response leaves an existing spec intact', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'generate_mod': {
        'ok': true,
        'files': {'enabled.txt': '', 'bad?name.txt': 'x'},
      },
    });
    final tmp = Directory.systemTemp.createTempSync('gore_mod_atomic_');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final existing = File(p.join(tmp.path, 'AtomicMod.spec.json'))
      ..writeAsStringSync('OLD');

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: ExportRequest(modName: 'AtomicMod', targetDir: tmp.path),
      overrides: [apple500],
    );

    expect(container.read(exportProvider).result?.success, isFalse);
    expect(existing.readAsStringSync(), 'OLD');
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
